# CLI Guide

Bitcoin Educational Implementation의 CLI 사용 가이드입니다.

## 목차

- [빌드 및 실행](#빌드-및-실행)
- [커맨드 레퍼런스](#커맨드-레퍼런스)
  - [init](#init)
  - [info](#info)
  - [mine](#mine)
  - [wallet new-address](#wallet-new-address)
  - [wallet list](#wallet-list)
  - [wallet balance](#wallet-balance)
  - [wallet send](#wallet-send)
  - [block get](#block-get)
  - [block height](#block-height)
  - [block best-block](#block-best-block)
- [사용 플로우](#사용-플로우)
  - [Flow 1: 기본 셋업](#flow-1-기본-셋업)
  - [Flow 2: 채굴 및 잔액 확인](#flow-2-채굴-및-잔액-확인)
  - [Flow 3: 코인 전송](#flow-3-코인-전송)
  - [Flow 4: 블록체인 탐색](#flow-4-블록체인-탐색)
- [데이터 저장 구조](#데이터-저장-구조)
- [에러 케이스](#에러-케이스)
- [내부 동작 개요](#내부-동작-개요)

---

## 빌드 및 실행

```bash
# 릴리스 빌드 (권장)
cargo build --release

# 실행 파일 경로
./target/release/bit-coin <COMMAND>

# 도움말
./target/release/bit-coin --help
```

> **데이터 경로**: 모든 데이터는 실행 디렉토리의 `./data/` 하위에 저장됩니다.

---

## 커맨드 레퍼런스

### `init`

블록체인을 초기화합니다. 제네시스 블록을 생성하고 UTXO 세트에 코인베이스 출력을 등록합니다.

```
bitcoin-edu init
```

**사전 조건**: 없음 (최초 실행 시 사용)

**출력 예시**:
```
Initializing blockchain...
✓ Genesis block created
  Hash: 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f
  Height: 0
```

**내부 동작**:
1. `Block::genesis()` 로 하드코딩된 제네시스 블록 생성
2. 블록을 `data/blocks/` sled DB에 저장
3. 높이 인덱스 (height=0 → hash) 저장
4. 체인 팁(tip) 해시 저장
5. 제네시스 코인베이스 출력을 `data/utxo/` UTXO 세트에 등록

> **주의**: `init`을 두 번 실행하면 제네시스 블록이 중복 등록됩니다. `data/` 디렉토리를 삭제 후 재실행하세요.

---

### `info`

현재 블록체인 상태를 출력합니다.

```
bitcoin-edu info
```

**사전 조건**: `init` 이 실행된 상태

**출력 예시**:
```
Blockchain Info:
  Height: 1
  Best block: 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f
  UTXO count: 3
```

| 필드 | 설명 |
|------|------|
| Height | 저장된 블록 수 (init 직후: 1) |
| Best block | 체인 팁 블록의 SHA256d 해시 (hex) |
| UTXO count | 현재 미사용 출력 수 |

---

### `mine`

새로운 블록을 PoW(Proof-of-Work)로 채굴합니다. 코인베이스 트랜잭션을 포함한 블록을 생성하고 블록체인에 저장합니다.

```
bitcoin-edu mine [--address <ADDRESS>]
```

| 옵션 | 필수 | 설명 |
|------|------|------|
| `--address` / `-a` | 선택 | 블록 보상을 받을 주소 (생략 시 기본 주소 사용) |

**출력 예시**:
```
Mining block 1...
  Found nonce 42381 in 42382 attempts (1823.4 KH/s)
Block mined successfully!
  Height:  1
  Hash:    00a3f7c2d1e8b4...
  Reward:  5000000000 satoshis (50.0 BTC) -> a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
```

**내부 동작**:
1. 현재 체인 팁(tip)과 높이를 조회
2. 보상 주소의 P2PKH scriptPubKey 생성
3. 블록 보상 50 BTC의 코인베이스 트랜잭션 생성
4. 머클 루트 계산 후 BlockHeader 구성 (bits: `0x20ffffff`, 교육용 쉬운 난이도)
5. PoW 마이닝: nonce 0부터 순차 탐색하여 목표 해시 충족 nonce 발견
6. 블록을 BlockchainDB에 저장 (block hash 인덱스, height 인덱스, tip 업데이트)
7. 코인베이스 출력을 UTXO 세트에 등록
8. DB flush (영속성 보장)

**에러 케이스**:
```
Error: Blockchain not initialized. Run 'init' first.
```
→ `init`을 먼저 실행하세요.

```
Error: No default address. Create one with 'wallet new-address'
```
→ `--address`를 지정하거나 먼저 지갑 주소를 생성하세요.

---

### `wallet new-address`

새로운 secp256k1 키 페어를 생성하고 주소를 반환합니다. 키는 `data/keystore.json`에 영속 저장됩니다.

```
bitcoin-edu wallet new-address
```

**출력 예시**:
```
New address: a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
```

**주소 형식**: 공개키의 HASH160(SHA256 → RIPEMD160) 결과를 hex로 인코딩한 40자 문자열

> **참고**: 첫 번째로 생성한 주소가 기본 주소(default address)로 설정됩니다. `wallet balance`, `wallet send`에서 주소를 생략하면 이 주소가 사용됩니다.

> **보안 주의**: 비밀키는 `data/keystore.json`에 평문으로 저장됩니다. 교육용 전용이며 실제 자산에 사용하지 마세요.

---

### `wallet list`

키스토어에 저장된 모든 주소를 출력합니다.

```
bitcoin-edu wallet list
```

**출력 예시**:
```
Addresses (2):
  a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
  9f8e7d6c5b4a9f8e7d6c5b4a9f8e7d6c5b4a9f8e
```

---

### `wallet balance`

특정 주소의 잔액을 UTXO 세트에서 조회합니다.

```
bitcoin-edu wallet balance [ADDRESS]
```

| 인자 | 필수 | 설명 |
|------|------|------|
| `ADDRESS` | 선택 | 조회할 주소 (생략 시 기본 주소 사용) |

**출력 예시**:
```
Balance for a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2:
  5000000000 satoshis (50.00000000 BTC)
```

**내부 동작**: UTXO 세트 전체를 스캔하여 주소의 scriptPubKey와 일치하는 출력의 합산값 반환

**에러 케이스**:
```
Error: No default address. Create one with 'wallet new-address'
```
→ 주소를 먼저 생성하세요.

---

### `wallet send`

기본 주소에서 지정한 주소로 코인을 전송하는 트랜잭션을 생성합니다.

```
bitcoin-edu wallet send <TO> <AMOUNT> [--fee <FEE>]
```

| 인자 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `TO` | 필수 | - | 수신자 주소 (40자 hex) |
| `AMOUNT` | 필수 | - | 전송할 금액 (단위: satoshi) |
| `--fee` / `-f` | 선택 | `1000` | 트랜잭션 수수료 (단위: satoshi) |

**출력 예시**:
```
Transaction created:
  TXID: 3a4b5c6d7e8f3a4b5c6d7e8f3a4b5c6d7e8f3a4b5c6d7e8f3a4b5c6d7e8f3a4b
  Inputs: 1
  Outputs: 2
  Total output: 49999000 satoshis
```

**출력 구성**:
- Output[0]: 수신자에게 전송되는 금액 (`AMOUNT` satoshi)
- Output[1]: 잔돈 (change) — 입력 합계 - `AMOUNT` - `fee` > 0 인 경우에만 생성

**잔돈 계산**:
```
change = total_input - amount - fee
```
`change > 0` 이면 송신자 주소로 돌아오는 잔돈 출력이 자동 생성됩니다.

**에러 케이스**:
```
Error: Insufficient funds: have 500, need 51000
```
→ UTXO의 합계가 `amount + fee`보다 부족합니다.

```
Error: No UTXOs available for sender
```
→ 해당 주소에 등록된 UTXO가 없습니다. 먼저 코인이 있어야 합니다.

> **참고**: 현재 구현에서 생성된 트랜잭션은 네트워크 브로드캐스트 없이 출력만 됩니다. 블록에 포함시키는 과정은 별도 구현이 필요합니다.

---

### `block get`

블록 높이 또는 블록 해시로 블록 정보를 조회합니다.

```
bitcoin-edu block get <ID>
```

| 인자 | 설명 |
|------|------|
| `ID` | 블록 높이(정수) 또는 블록 해시(64자 hex) |

**높이로 조회**:
```bash
bitcoin-edu block get 0
```
```
Block:
  Hash: 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f
  Previous: 0000000000000000000000000000000000000000000000000000000000000000
  Merkle root: 4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b
  Timestamp: 1231006505
  Nonce: 2083236893
  Transactions: 1
    [0] 4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b
```

**해시로 조회**:
```bash
bitcoin-edu block get 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f
```

**출력 필드**:

| 필드 | 설명 |
|------|------|
| Hash | 이 블록의 SHA256d 해시 |
| Previous | 이전 블록 해시 (제네시스는 모두 0) |
| Merkle root | 트랜잭션들의 머클 트리 루트 |
| Timestamp | Unix 타임스탬프 (초) |
| Nonce | PoW 마이닝에서 찾은 논스 |
| Transactions | 포함된 트랜잭션 수 및 TXID 목록 |

**에러 케이스**:
```
Error: Block not found: 99
```

---

### `block height`

현재 저장된 블록 수(체인 높이)를 출력합니다.

```
bitcoin-edu block height
```

**출력 예시**:
```
Blockchain height: 1
```

> **참고**: `init` 직후 값은 `1`입니다. 내부적으로 `store_chain_height(1)`이 호출되어 제네시스 블록 1개가 저장된 상태를 나타냅니다.

---

### `block best-block`

가장 최근 블록(체인 팁)의 해시를 출력합니다.

```
bitcoin-edu block best-block
```

**출력 예시**:
```
Best block: 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f
```

---

## 사용 플로우

### Flow 1: 기본 셋업

블록체인 초기화 후 지갑을 준비하는 기본 흐름입니다.

```bash
# 1. 블록체인 초기화 (제네시스 블록 생성)
$ ./target/release/bit-coin init
Initializing blockchain...
✓ Genesis block created
  Hash: 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f
  Height: 0

# 2. 현재 상태 확인
$ ./target/release/bit-coin info
Blockchain Info:
  Height: 1
  Best block: 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f
  UTXO count: 1

# 3. 지갑 주소 생성
$ ./target/release/bit-coin wallet new-address
New address: a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2

# 4. 주소 목록 확인
$ ./target/release/bit-coin wallet list
Addresses (1):
  a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2

# 5. 잔액 확인 (초기에는 0)
$ ./target/release/bit-coin wallet balance
Balance for a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2:
  0 satoshis (0.00000000 BTC)
```

---

### Flow 2: 채굴 및 잔액 확인

지갑 주소를 만든 뒤 블록을 채굴하여 보상을 받는 흐름입니다.

```bash
# 1. 지갑 주소 생성 (채굴 보상을 받을 주소)
$ ./target/release/bit-coin wallet new-address
New address: a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2

# 2. 첫 번째 블록 채굴 (기본 주소로 보상)
$ ./target/release/bit-coin mine
Mining block 1...
  Found nonce 42381 in 42382 attempts (1823.4 KH/s)
Block mined successfully!
  Height:  1
  Hash:    00a3f7c2d1e8b4...
  Reward:  5000000000 satoshis (50.0 BTC) -> a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2

# 3. 잔액 확인 (50 BTC 증가)
$ ./target/release/bit-coin wallet balance
Balance for a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2:
  5000000000 satoshis (50.00000000 BTC)

# 4. 특정 주소로 보상을 보내면서 채굴
$ ./target/release/bit-coin mine --address 9f8e7d6c5b4a9f8e7d6c5b4a9f8e7d6c5b4a9f8e
Mining block 2...
  Found nonce 11203 in 11204 attempts (2104.1 KH/s)
Block mined successfully!
  Height:  2
  Hash:    003a1c9e...
  Reward:  5000000000 satoshis (50.0 BTC) -> 9f8e7d6c5b4a9f8e7d6c5b4a9f8e7d6c5b4a9f8e

# 5. 블록체인 상태 확인
$ ./target/release/bit-coin info
Blockchain Info:
  Height: 3
  Best block: 003a1c9e...
  UTXO count: 3
```

---

### Flow 3: 코인 전송

채굴로 얻은 코인을 다른 주소로 전송하는 흐름입니다.

```bash
# 1. 수신자 주소 생성
$ ./target/release/bit-coin wallet new-address
New address: 9f8e7d6c5b4a9f8e7d6c5b4a9f8e7d6c5b4a9f8e

# 2. 송신자 잔액 확인
$ ./target/release/bit-coin wallet balance
Balance for a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2:
  5000000000 satoshis (50.00000000 BTC)

# 3. 코인 전송 (기본 수수료: 1000 satoshi)
$ ./target/release/bit-coin wallet send 9f8e7d6c5b4a9f8e7d6c5b4a9f8e7d6c5b4a9f8e 100000000
Transaction created:
  TXID: 3a4b5c6d7e8f3a4b5c6d7e8f3a4b5c6d7e8f3a4b5c6d7e8f3a4b5c6d7e8f3a4b
  Inputs: 1
  Outputs: 2
  Total output: 4999999000 satoshis

# 4. 수수료를 직접 지정하는 경우
$ ./target/release/bit-coin wallet send 9f8e7d6c5b4a9f8e7d6c5b4a9f8e7d6c5b4a9f8e 100000000 --fee 5000

# 트랜잭션 구조:
#   Input:  a1b2... 주소의 UTXO 5,000,000,000 satoshi
#   Output[0]: 9f8e... 주소로  100,000,000 satoshi (수신자)
#   Output[1]: a1b2... 주소로 4,899,999,000 satoshi (잔돈)
```

**전송 금액 계산 다이어그램**:

```
UTXO (5,000,000,000 sat)
       │
       ▼
  ┌──────────────────────────────────────────┐
  │              Transaction                 │
  │  Input:  5,000,000,000 sat               │
  │  Output[0] → 수신자:   100,000,000 sat   │
  │  Output[1] → 잔돈:   4,899,999,000 sat   │
  │  Fee:                        1,000 sat   │
  └──────────────────────────────────────────┘
```

---

### Flow 4: 블록체인 탐색

채굴된 블록들을 탐색하는 흐름입니다.

```bash
# 1. 현재 체인 높이 확인
$ ./target/release/bit-coin block height
Blockchain height: 1

# 2. 현재 체인 팁 해시 확인
$ ./target/release/bit-coin block best-block
Best block: 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f

# 3. 제네시스 블록을 높이로 조회
$ ./target/release/bit-coin block get 0
Block:
  Hash: 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f
  Previous: 0000000000000000000000000000000000000000000000000000000000000000
  Merkle root: 4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b
  Timestamp: 1231006505
  Nonce: 2083236893
  Transactions: 1
    [0] 4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b

# 4. 동일한 블록을 해시로 조회
$ ./target/release/bit-coin block get 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f
Block:
  Hash: 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f
  ...

# 5. 전체 상태 재확인
$ ./target/release/bit-coin info
Blockchain Info:
  Height: 1
  Best block: 000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f
  UTXO count: 1
```

---

## 데이터 저장 구조

```
./data/
├── blocks/          # 블록체인 DB (sled embedded)
│   └── ...          # 블록 해시 → 직렬화된 블록 데이터
│                    # 높이 인덱스: height → hash
│                    # tip, height 메타데이터 키
│
├── utxo/            # UTXO 세트 (sled embedded)
│   └── ...          # OutPoint(txid+vout) → UTXO(output+height+coinbase flag)
│
└── keystore.json    # 지갑 키스토어 (JSON 평문)
                     # { address → { secret_key_bytes, address } }
```

**keystore.json 예시**:
```json
{
  "keys": {
    "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2": {
      "secret_key_bytes": [12, 34, 56, ...],
      "address": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
    }
  },
  "default_address": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
}
```

> **경고**: `keystore.json`에는 비밀키가 암호화 없이 저장됩니다. 교육 목적 전용입니다.

---

## 에러 케이스

| 에러 메시지 | 원인 | 해결 방법 |
|------------|------|----------|
| `Blockchain not initialized. Run 'init' first.` | init 미실행 | `init` 먼저 실행 |
| `No default address. Create one with 'wallet new-address'` | 지갑 주소 없음 | `wallet new-address` 실행 |
| `Insufficient funds: have X, need Y` | UTXO 잔액 부족 | 전송 금액 또는 수수료를 줄이거나 잔액을 충전 |
| `No UTXOs available for sender` | 해당 주소에 UTXO 없음 | 코인이 있는 주소를 사용하거나 잔액 충전 |
| `Address not found in keystore` | 잔액 조회 주소가 키스토어에 없음 | 본인이 생성한 주소만 잔액 조회 가능 |
| `Block not found: X` | 해당 높이/해시의 블록 없음 | `block height`로 현재 높이 확인 후 재시도 |
| `Error initializing: ...` | `data/` 디렉토리 접근 오류 | 실행 디렉토리 쓰기 권한 확인 |

---

## 내부 동작 개요

### 트랜잭션 서명 흐름

`wallet send` 명령이 실행될 때 내부에서 발생하는 암호화 흐름:

```
wallet send <TO> <AMOUNT>
       │
       ▼
1. Keystore에서 기본 주소의 KeyPair 로드
       │
       ▼
2. UTXO 세트에서 scriptPubKey가 일치하는 UTXO 목록 조회
       │
       ▼
3. UTXO 선택 (amount + fee를 충족할 때까지 순서대로 선택)
       │
       ▼
4. 트랜잭션 구성
   - Input: 선택된 UTXO의 OutPoint (txid + vout)
   - Output[0]: 수신자 P2PKH scriptPubKey + amount
   - Output[1]: 송신자 P2PKH scriptPubKey + change (있는 경우)
       │
       ▼
5. 트랜잭션 서명 (ECDSA with secp256k1)
   - tx.txid() → 32바이트 해시 생성
   - secp256k1::sign_ecdsa(message, secret_key)
   - DER 포맷 직렬화
   - scriptSig = <sig_len><sig><pubkey_len><pubkey>
       │
       ▼
6. 완성된 트랜잭션 출력 (TXID, 입력/출력 수, 총 출력값)
```

### P2PKH 스크립트 구조

이 구현에서 모든 주소는 P2PKH(Pay-to-Public-Key-Hash) 방식을 사용합니다:

```
scriptPubKey (잠금):
  OP_DUP OP_HASH160 <pubKeyHash(20바이트)> OP_EQUALVERIFY OP_CHECKSIG
  76     a9         14 <...20 bytes...>    88             ac
  (총 25바이트)

scriptSig (해제):
  <sig_len(1바이트)> <DER서명> <pubkey_len(1바이트)> <압축공개키(33바이트)>
```

### UTXO 모델

잔액은 계좌 잔고가 아닌 UTXO(미사용 트랜잭션 출력)의 합으로 관리됩니다:

```
잔액 계산:
  balance = Σ utxo.value  (for all utxo where utxo.scriptPubKey == 내 scriptPubKey)

이중 지불 방지:
  트랜잭션 생성 시 사용된 UTXO는 소비됨(spent)으로 처리되어 UTXO 세트에서 제거
```
