// GPU-accelerated Proof-of-Work using wgpu compute shaders
//
// Architecture:
//   - Each GPU thread tries one nonce value (start_nonce + thread_id)
//   - WGSL compute shader implements full SHA256d (double SHA256) on-GPU
//   - Results are read back via a staging buffer
//   - Automatically falls back to CPU if no GPU adapter is found

use crate::core::BlockHeader;
use crate::consensus::pow::{Miner, MiningResult, Target};
use std::time::Instant;

/// Number of threads per workgroup (must match @workgroup_size in WGSL)
const WORKGROUP_SIZE: u32 = 256;
/// Number of workgroups dispatched per batch → 256 * 4096 = 1,048,576 nonces
const GROUPS_PER_DISPATCH: u32 = 4096;

// ── GPU buffer layouts ──────────────────────────────────────────────────────

/// Parameters written to the GPU once per dispatch batch.
/// `header_prefix` holds header bytes 0-75 (version through bits) as
/// little-endian u32 words; the nonce at bytes 76-79 is supplied by the shader.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuParams {
    /// Block header bytes 0-75 packed as 19 little-endian u32 words
    header_prefix: [u32; 19],
    /// SHA256 target as 8 big-endian u32 words (for direct comparison)
    target_be: [u32; 8],
    /// First nonce value this batch will try
    start_nonce: u32,
    /// Padding to keep struct size a multiple of 16 bytes (wgpu requirement)
    _pad: u32,
}

/// Result written back from the GPU.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuResult {
    /// 1 if a valid nonce was found, 0 otherwise
    found: u32,
    /// The valid nonce (little-endian u32, same as header.nonce)
    nonce: u32,
}

// ── WGSL compute shader ─────────────────────────────────────────────────────

const SHADER_SRC: &str = r#"
// ── Bindings ─────────────────────────────────────────────────────────────────

struct Params {
    header_prefix : array<u32, 19>,  // bytes 0-75 as little-endian u32
    target_be     : array<u32,  8>,  // target as big-endian u32
    start_nonce   : u32,
    _pad          : u32,
}

struct Result {
    found : u32,
    nonce : u32,
}

@group(0) @binding(0) var<storage, read>       params : Params;
@group(0) @binding(1) var<storage, read_write> result : Result;

// ── SHA256 constants ──────────────────────────────────────────────────────────
// Note: declared as functions returning var locals so naga allows dynamic indexing.

fn sha256_h0() -> array<u32, 8> {
    var h : array<u32, 8> = array<u32, 8>(
        0x6a09e667u, 0xbb67ae85u, 0x3c6ef372u, 0xa54ff53au,
        0x510e527fu, 0x9b05688cu, 0x1f83d9abu, 0x5be0cd19u,
    );
    return h;
}

fn sha256_k() -> array<u32, 64> {
    var k : array<u32, 64> = array<u32, 64>(
        0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u,
        0x3956c25bu, 0x59f111f1u, 0x923f82a4u, 0xab1c5ed5u,
        0xd807aa98u, 0x12835b01u, 0x243185beu, 0x550c7dc3u,
        0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u, 0xc19bf174u,
        0xe49b69c1u, 0xefbe4786u, 0x0fc19dc6u, 0x240ca1ccu,
        0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau,
        0x983e5152u, 0xa831c66du, 0xb00327c8u, 0xbf597fc7u,
        0xc6e00bf3u, 0xd5a79147u, 0x06ca6351u, 0x14292967u,
        0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu, 0x53380d13u,
        0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u,
        0xa2bfe8a1u, 0xa81a664bu, 0xc24b8b70u, 0xc76c51a3u,
        0xd192e819u, 0xd6990624u, 0xf40e3585u, 0x106aa070u,
        0x19a4c116u, 0x1e376c08u, 0x2748774cu, 0x34b0bcb5u,
        0x391c0cb3u, 0x4ed8aa4au, 0x5b9cca4fu, 0x682e6ff3u,
        0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u,
        0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u,
    );
    return k;
}

// ── SHA256 helpers ────────────────────────────────────────────────────────────

fn rotr(x: u32, n: u32) -> u32 {
    return (x >> n) | (x << (32u - n));
}

// Byte-swap a little-endian u32 to big-endian for SHA256 message schedule
fn swap(x: u32) -> u32 {
    return ((x & 0xFFu)       << 24u) |
           ((x & 0xFF00u)     <<  8u) |
           ((x >> 8u)  & 0xFF00u)     |
           ((x >> 24u) & 0xFFu);
}

// One SHA256 compression round.
// Accepts the running hash state (8 words) and one 16-word message block.
// Returns the updated state.
fn compress(state_in: array<u32, 8>, blk_in: array<u32, 16>) -> array<u32, 8> {
    // var locals allow dynamic indexing in naga
    var state = state_in;
    var blk   = blk_in;
    var K     = sha256_k();

    var w : array<u32, 64>;
    for (var i = 0u; i < 16u; i++) { w[i] = blk[i]; }
    for (var i = 16u; i < 64u; i++) {
        let s0 = rotr(w[i-15u], 7u) ^ rotr(w[i-15u], 18u) ^ (w[i-15u] >> 3u);
        let s1 = rotr(w[i- 2u],17u) ^ rotr(w[i- 2u], 19u) ^ (w[i- 2u] >> 10u);
        w[i] = w[i-16u] + s0 + w[i-7u] + s1;
    }

    var a = state[0]; var b = state[1]; var c = state[2]; var d = state[3];
    var e = state[4]; var f = state[5]; var g = state[6]; var h = state[7];

    for (var i = 0u; i < 64u; i++) {
        let S1   = rotr(e, 6u) ^ rotr(e, 11u) ^ rotr(e, 25u);
        let ch   = (e & f) ^ (~e & g);
        let t1   = h + S1 + ch + K[i] + w[i];
        let S0   = rotr(a, 2u) ^ rotr(a, 13u) ^ rotr(a, 22u);
        let maj  = (a & b) ^ (a & c) ^ (b & c);
        let t2   = S0 + maj;
        h = g; g = f; f = e; e = d + t1;
        d = c; c = b; b = a; a = t1 + t2;
    }

    return array<u32, 8>(
        state[0]+a, state[1]+b, state[2]+c, state[3]+d,
        state[4]+e, state[5]+f, state[6]+g, state[7]+h,
    );
}

// ── SHA256d of the 80-byte block header ───────────────────────────────────────
//
// nonce_le  : the nonce as a Rust little-endian u32 (bytes 76-79 of header)
//
// Byte layout of a Bitcoin block header:
//   bytes  0- 3 : version          → header_prefix[0]
//   bytes  4-35 : prev_block_hash  → header_prefix[1..9]
//   bytes 36-67 : merkle_root      → header_prefix[9..17]
//   bytes 68-71 : timestamp        → header_prefix[17]
//   bytes 72-75 : bits             → header_prefix[18]
//   bytes 76-79 : nonce            → nonce_le  (varied by this shader)
//
// SHA256 processes big-endian 32-bit words, so each LE u32 must be
// byte-swapped before entering the message schedule.

fn sha256d(nonce_le: u32) -> array<u32, 8> {

    // ── Pass 1, Block 1: header bytes 0-63 (16 LE u32 → 16 BE u32) ──────────
    // Copy storage buffer fields into var locals for dynamic indexing
    var prefix : array<u32, 19>;
    prefix[ 0] = params.header_prefix[ 0];
    prefix[ 1] = params.header_prefix[ 1];
    prefix[ 2] = params.header_prefix[ 2];
    prefix[ 3] = params.header_prefix[ 3];
    prefix[ 4] = params.header_prefix[ 4];
    prefix[ 5] = params.header_prefix[ 5];
    prefix[ 6] = params.header_prefix[ 6];
    prefix[ 7] = params.header_prefix[ 7];
    prefix[ 8] = params.header_prefix[ 8];
    prefix[ 9] = params.header_prefix[ 9];
    prefix[10] = params.header_prefix[10];
    prefix[11] = params.header_prefix[11];
    prefix[12] = params.header_prefix[12];
    prefix[13] = params.header_prefix[13];
    prefix[14] = params.header_prefix[14];
    prefix[15] = params.header_prefix[15];
    prefix[16] = params.header_prefix[16];
    prefix[17] = params.header_prefix[17];
    prefix[18] = params.header_prefix[18];

    var blk1 : array<u32, 16>;
    for (var i = 0u; i < 16u; i++) { blk1[i] = swap(prefix[i]); }

    var st = sha256_h0();
    st = compress(st, blk1);

    // ── Pass 1, Block 2: header bytes 64-79 + SHA256 padding ─────────────────
    // bytes 64-67: header_prefix[16]  (timestamp lower or bits depending on layout)
    // bytes 68-71: header_prefix[17]
    // bytes 72-75: header_prefix[18]
    // bytes 76-79: nonce_le
    // padding : 0x80 byte then zeros, then 64-bit big-endian bit-length = 640
    var blk2 : array<u32, 16>;
    blk2[0]  = swap(prefix[16]);
    blk2[1]  = swap(prefix[17]);
    blk2[2]  = swap(prefix[18]);
    blk2[3]  = swap(nonce_le);
    blk2[4]  = 0x80000000u;  // 0x80 padding byte
    blk2[5]  = 0u; blk2[6]  = 0u; blk2[7]  = 0u;
    blk2[8]  = 0u; blk2[9]  = 0u; blk2[10] = 0u;
    blk2[11] = 0u; blk2[12] = 0u; blk2[13] = 0u;
    blk2[14] = 0u;     // high 32 bits of bit-length (640 < 2^32, so 0)
    blk2[15] = 640u;   // low  32 bits: 80 bytes × 8 = 640

    var hash1 = compress(st, blk2);

    // ── Pass 2: SHA256 of the 32-byte intermediate hash ───────────────────────
    // The 8 u32 words from hash1 are already big-endian SHA256 output.
    // Padding: 0x80, zeros, bit-length = 256
    var blk3 : array<u32, 16>;
    for (var i = 0u; i < 8u; i++) { blk3[i] = hash1[i]; }
    blk3[8]  = 0x80000000u;
    blk3[9]  = 0u; blk3[10] = 0u; blk3[11] = 0u;
    blk3[12] = 0u; blk3[13] = 0u;
    blk3[14] = 0u;     // high 32 bits of 256
    blk3[15] = 256u;   // low  32 bits: 32 bytes × 8 = 256

    return compress(sha256_h0(), blk3);
}

// ── Entry point ───────────────────────────────────────────────────────────────

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid : vec3<u32>) {
    let nonce = params.start_nonce + gid.x;
    var hash  = sha256d(nonce);

    // Copy storage target into a var local for dynamic indexing
    var tgt : array<u32, 8>;
    tgt[0] = params.target_be[0];
    tgt[1] = params.target_be[1];
    tgt[2] = params.target_be[2];
    tgt[3] = params.target_be[3];
    tgt[4] = params.target_be[4];
    tgt[5] = params.target_be[5];
    tgt[6] = params.target_be[6];
    tgt[7] = params.target_be[7];

    // Check hash < tgt  (both in big-endian u32 order)
    var below = false;
    for (var i = 0u; i < 8u; i++) {
        if hash[i] < tgt[i] { below = true; break; }
        if hash[i] > tgt[i] {               break; }
    }

    if below && result.found == 0u {
        result.found = 1u;
        result.nonce = nonce;
    }
}
"#;

// ── GpuMiner ─────────────────────────────────────────────────────────────────

/// GPU miner backed by wgpu compute shaders.
/// Falls back to the CPU `Miner` automatically if no GPU adapter is available.
pub struct GpuMiner {
    bits: u32,
    target_be: [u32; 8],
}

impl GpuMiner {
    /// Create a new GPU miner for the given compact-format difficulty `bits`.
    pub fn new(bits: u32) -> Self {
        let target = Target::from_bits(bits);
        let target_bytes = target.to_hash256();
        let tb = target_bytes.as_bytes();

        // Convert target byte array → 8 big-endian u32 for direct GPU comparison
        let mut target_be = [0u32; 8];
        for i in 0..8 {
            target_be[i] = u32::from_be_bytes([
                tb[i * 4],
                tb[i * 4 + 1],
                tb[i * 4 + 2],
                tb[i * 4 + 3],
            ]);
        }
        Self { bits, target_be }
    }

    /// Mine a block header. Tries GPU first, falls back to CPU on any error.
    pub fn mine(&self, header: &mut BlockHeader) -> MiningResult {
        match self.mine_gpu(header) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("GPU mining unavailable ({}), falling back to CPU", e);
                let cpu = Miner::new(self.bits);
                cpu.mine(header)
            }
        }
    }

    fn mine_gpu(&self, header: &mut BlockHeader) -> Result<MiningResult, String> {
        // ── Initialise wgpu ───────────────────────────────────────────────────
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            },
        ))
        .ok_or("No GPU adapter found – is a GPU driver installed?")?;

        let adapter_info = adapter.get_info();
        log::info!(
            "GPU: {} ({:?})",
            adapter_info.name,
            adapter_info.backend
        );

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("bitcoin-mining"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        ))
        .map_err(|e| format!("Failed to create GPU device: {}", e))?;

        // ── Extract header prefix (bytes 0-75 as 19 LE u32 words) ────────────
        let raw = header.serialize_to_array();
        let mut header_prefix = [0u32; 19];
        for i in 0..19 {
            header_prefix[i] = u32::from_le_bytes([
                raw[i * 4],
                raw[i * 4 + 1],
                raw[i * 4 + 2],
                raw[i * 4 + 3],
            ]);
        }

        // ── Create shader & pipeline ──────────────────────────────────────────
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sha256d"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mining_bgl"),
            entries: &[
                // binding 0: params (read-only storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: result (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mining_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("mining_cp"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });

        // ── Allocate GPU buffers ──────────────────────────────────────────────
        use std::mem::size_of;

        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("params"),
            size: size_of::<GpuParams>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let result_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("result"),
            size: size_of::<GpuResult>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: size_of::<GpuResult>() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mining_bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: result_buf.as_entire_binding(),
                },
            ],
        });

        // ── Mining loop ───────────────────────────────────────────────────────
        let nonces_per_dispatch = WORKGROUP_SIZE * GROUPS_PER_DISPATCH; // 1,048,576
        let start_time = Instant::now();
        let mut total_attempts: u64 = 0;
        let mut start_nonce: u32 = 0;

        log::info!(
            "GPU dispatch: {} workgroups × {} threads = {} nonces/batch",
            GROUPS_PER_DISPATCH,
            WORKGROUP_SIZE,
            nonces_per_dispatch
        );

        loop {
            // Write params for this batch
            let gpu_params = GpuParams {
                header_prefix,
                target_be: self.target_be,
                start_nonce,
                _pad: 0,
            };
            queue.write_buffer(&params_buf, 0, bytemuck::bytes_of(&gpu_params));

            // Clear result buffer
            let zero_result = GpuResult { found: 0, nonce: 0 };
            queue.write_buffer(&result_buf, 0, bytemuck::bytes_of(&zero_result));

            // Record and submit compute commands
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("mining_cmd"),
                });
            {
                let mut pass =
                    encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("mining_pass"),
                        timestamp_writes: None,
                    });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(GROUPS_PER_DISPATCH, 1, 1);
            }
            encoder.copy_buffer_to_buffer(
                &result_buf,
                0,
                &staging_buf,
                0,
                size_of::<GpuResult>() as u64,
            );
            queue.submit(std::iter::once(encoder.finish()));

            // Read back result (blocking)
            let buf_slice = staging_buf.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buf_slice.map_async(wgpu::MapMode::Read, move |v| {
                tx.send(v).unwrap();
            });
            device.poll(wgpu::Maintain::Wait);
            rx.recv()
                .map_err(|_| "GPU readback channel closed".to_string())?
                .map_err(|e| format!("GPU buffer map failed: {:?}", e))?;

            let gpu_result: GpuResult = {
                let view = buf_slice.get_mapped_range();
                *bytemuck::from_bytes::<GpuResult>(&view)
            };
            staging_buf.unmap();

            total_attempts += nonces_per_dispatch as u64;

            if gpu_result.found != 0 {
                // CPU-side verification: set nonce and re-hash
                header.nonce = gpu_result.nonce;
                let hash = header.hash();
                let target = Target::from_bits(self.bits);

                if target.is_valid_hash(&hash) {
                    let elapsed = start_time.elapsed();
                    return Ok(MiningResult {
                        success: true,
                        nonce: gpu_result.nonce,
                        hash,
                        attempts: total_attempts,
                        duration: elapsed,
                    });
                }
                // Rare race: two threads found simultaneously; scan next few on CPU
                let cpu = Miner::new(self.bits);
                for offset in 1..=256u32 {
                    header.nonce = gpu_result.nonce.wrapping_add(offset);
                    if target.is_valid_hash(&header.hash()) {
                        let elapsed = start_time.elapsed();
                        return Ok(MiningResult {
                            success: true,
                            nonce: header.nonce,
                            hash: header.hash(),
                            attempts: total_attempts + offset as u64,
                            duration: elapsed,
                        });
                    }
                }
                // If still not found, continue GPU batches from next range
                let _ = cpu; // suppress unused warning
            }

            // Progress log
            let elapsed = start_time.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                log::debug!(
                    "GPU mining: {} Mnonces ({:.1} MH/s)",
                    total_attempts / 1_000_000,
                    total_attempts as f64 / elapsed / 1_000_000.0
                );
            }

            // Advance to next batch, detect u32 overflow (all nonces exhausted)
            start_nonce = match start_nonce.checked_add(nonces_per_dispatch) {
                Some(n) => n,
                None => {
                    let elapsed = start_time.elapsed();
                    return Ok(MiningResult {
                        success: false,
                        nonce: 0,
                        hash: crate::core::Hash256::zero(),
                        attempts: total_attempts,
                        duration: elapsed,
                    });
                }
            };
        }
    }
}
