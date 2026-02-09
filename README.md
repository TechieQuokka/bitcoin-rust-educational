# Bitcoin Educational Implementation

2009년 사토시 나카모토의 Bitcoin v0.1.0을 Rust로 재구현하는 교육용 프로젝트

## 프로젝트 개요

이 프로젝트는 블록체인 핵심 개념을 학습하기 위해 Bitcoin의 초기 버전을 Rust로 구현합니다.
실제 Bitcoin 네트워크와 호환되지 않는 독립적인 교육용 체인입니다.

### 주요 특징

- **핵심 로직 충실**: 합의 알고리즘과 트랜잭션 검증은 2009년 방식 그대로
- **현대적 구현**: Rust의 안전성과 성능을 활용한 모범 사례 적용
- **단계별 구현**: Bottom-up 방식으로 기초부터 체계적 구축

## 현재 상태: 전체 완성! ✅

### 구현된 기능

**Phase 1 - 기초 데이터 구조** ✅
- Block, BlockHeader, Transaction
- SHA256 double hash, Merkle tree
- VarInt 직렬화, 제네시스 블록

**Phase 2 - 검증 & 암호화** ✅
- Proof of Work (난이도 고정)
- P2PKH Bitcoin Script
- ECDSA 서명 검증 (secp256k1)
- 블록/트랜잭션 검증

**Phase 3 - 저장소** ✅
- Blockchain DB (sled)
- UTXO set 관리
- 높이 인덱싱, 잔액 계산

**Phase 4 - P2P 네트워크** ✅
- 프로토콜 메시지 (Version, Ping, Inv 등)
- Peer 연결 관리 (tokio)
- Network node 구조

**Phase 5 - 지갑 & CLI** ✅
- 키 관리 (Keystore)
- 트랜잭션 빌더
- 완전한 CLI 도구

### 테스트 결과

```
✅ 56개 단위 테스트 통과
✅ 전체 통합 테스트 통과
✅ CLI 명령어 작동 확인
✅ 11/11 CLI 시나리오 테스트 통과 (100%)
```

### 성능 최적화

**주요 개선 사항**:
- ✅ Database flush 최적화 (10-100배 향상)
- ✅ 마이닝 메모리 할당 제거 (76MB 절감/100만 시도)
- ✅ Target 캐싱으로 반복 변환 제거
- ✅ 블록 역직렬화 O(n²) → O(n) 개선
- ✅ Keystore 영속성 추가 (JSON 저장)

## 빌드 및 실행

### 필요 사항

- Rust 1.70 이상
- Cargo

### 빌드

```bash
cargo build --release
```

### 테스트 실행

```bash
cargo test
```

### CLI 사용 예제

#### 완전한 사용 시나리오

```bash
# 1. 블록체인 초기화
./target/release/bit-coin init
# ✓ Genesis block created
# Hash: 760082c6f7a0200c6bba73404a3ef4ccaf4f5fbcdbb85eb50cccdeff6d5e6520

# 2. 새 지갑 주소 생성
./target/release/bit-coin wallet new-address
# New address: 8fb8050e547ccdbd53f836a93230b9fabfbdb655

# 3. 주소 목록 확인
./target/release/bit-coin wallet list
# Addresses (1):
#   8fb8050e547ccdbd53f836a93230b9fabfbdb655

# 4. 잔액 조회
./target/release/bit-coin wallet balance
# Balance for 8fb8050e547ccdbd53f836a93230b9fabfbdb655:
#   0 satoshis (0 BTC)

# 5. 블록체인 정보
./target/release/bit-coin info
# Blockchain Info:
#   Height: 1
#   Best block: 760082c6f7a0200c6bba73404a3ef4ccaf4f5fbcdbb85eb50cccdeff6d5e6520
#   UTXO count: 1

# 6. Genesis 블록 조회
./target/release/bit-coin block get 0
# Block:
#   Hash: 760082c6f7a0200c6bba73404a3ef4ccaf4f5fbcdbb85eb50cccdeff6d5e6520
#   Previous: 0000000000000000000000000000000000000000000000000000000000000000
#   Transactions: 1
```

**주요 특징**:
- ✅ 모든 데이터 영구 저장 (블록체인, UTXO, 지갑)
- ✅ 재시작 후에도 완전한 상태 유지
- ✅ 실시간 잔액 조회 및 트랜잭션 관리

#### 블록체인 초기화
```bash
./target/release/bit-coin init
```

#### 블록체인 정보 조회
```bash
./target/release/bit-coin info
```

#### 지갑 명령어
```bash
# 새 주소 생성
./target/release/bit-coin wallet new-address

# 주소 목록 조회
./target/release/bit-coin wallet list

# 잔액 조회
./target/release/bit-coin wallet balance

# 송금
./target/release/bit-coin wallet send <주소> <금액> --fee <수수료>
```

#### 블록 명령어
```bash
# 블록 조회 (높이 또는 해시)
./target/release/bit-coin block get 0

# 체인 높이 조회
./target/release/bit-coin block height

# 최신 블록 조회
./target/release/bit-coin block best-block
```

### 데모 실행

```bash
cargo run --example demo
```

### 데이터 저장 위치

프로젝트는 실행 디렉토리에 `data/` 폴더를 생성하여 모든 데이터를 저장합니다:

```
data/
├── blocks/          # 블록체인 데이터베이스 (sled)
├── utxo/            # UTXO 세트 (sled)
└── keystore.json    # 지갑 키 저장소 (JSON)
```

**보안 주의사항**: `keystore.json`에는 개인키가 평문으로 저장됩니다. 교육 목적으로만 사용하세요.

## 프로젝트 구조

```
src/
├── lib.rs              # 라이브러리 루트
├── main.rs             # CLI 진입점
├── cli.rs              # CLI 명령어 ✓
├── core/               # 핵심 데이터 구조 ✓
│   ├── types.rs        # Hash256 등 기본 타입
│   ├── hash.rs         # 해싱 유틸리티
│   ├── serialize.rs    # 직렬화 유틸리티
│   ├── script.rs       # Bitcoin Script (P2PKH)
│   ├── transaction.rs  # 트랜잭션 구조
│   └── block.rs        # 블록 구조
├── consensus/          # 합의 & 검증 ✓
│   ├── pow.rs          # Proof of Work
│   └── validation.rs   # 블록/트랜잭션 검증
├── storage/            # 저장소 ✓
│   ├── blockchain_db.rs # 블록체인 DB
│   └── utxo_set.rs     # UTXO 관리
├── network/            # P2P 네트워크 ✓
│   ├── message.rs      # 프로토콜 메시지
│   ├── peer.rs         # Peer 연결
│   └── node.rs         # Network node
└── wallet/             # 지갑 ✓
    ├── keystore.rs     # 키 관리
    └── tx_builder.rs   # 트랜잭션 빌더

examples/
├── demo.rs             # 전체 기능 데모
└── mine_genesis.rs     # 제네시스 블록 채굴
```

## 완성된 기능

### ✅ 모든 Phase 완료

- ✅ Phase 1: 기초 데이터 구조
- ✅ Phase 2: 검증 & 암호화
- ✅ Phase 3: 저장소
- ✅ Phase 4: P2P 네트워크
- ✅ Phase 5: 지갑 & CLI

### 추가 개선 가능 영역

- [ ] 실제 P2P 네트워크 연결
- [ ] 채굴 풀 지원
- [ ] HD Wallet (BIP32/BIP44)
- [ ] GUI 인터페이스
- [ ] REST API 서버

## 기술 스택

### 직접 구현
- 블록/트랜잭션 구조
- PoW 합의
- UTXO 모델
- Merkle tree

### 라이브러리 사용
- `sha2` - SHA256 해싱
- `secp256k1` - ECDSA 암호화
- `ripemd` - RIPEMD160 해싱
- `serde` - 직렬화
- `tokio` - 비동기 런타임
- `sled` - 데이터베이스
- `clap` - CLI

## 참고 문서

- [프로젝트 개요](docs/PROJECT_OVERVIEW.md) - 목표 및 방향성
- [아키텍처](docs/ARCHITECTURE.md) - 시스템 설계
- [개발 단계](docs/DEVELOPMENT_PHASES.md) - 단계별 계획

## 라이센스

Educational purposes only
