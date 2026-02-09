# Bitcoin Educational Implementation

2009년 사토시 나카모토의 Bitcoin v0.1.0을 Rust로 재구현하는 교육용 프로젝트

## 프로젝트 개요

이 프로젝트는 블록체인 핵심 개념을 학습하기 위해 Bitcoin의 초기 버전을 Rust로 구현합니다.
실제 Bitcoin 네트워크와 호환되지 않는 독립적인 교육용 체인입니다.

### 주요 특징

- **핵심 로직 충실**: 합의 알고리즘과 트랜잭션 검증은 2009년 방식 그대로
- **현대적 구현**: Rust의 안전성과 성능을 활용한 모범 사례 적용
- **단계별 구현**: Bottom-up 방식으로 기초부터 체계적 구축

## 현재 상태: Phase 1 완료 ✓

### 구현된 기능

- ✅ **기초 데이터 구조**
  - Block, BlockHeader
  - Transaction, TxInput, TxOutput
  - Hash256 타입

- ✅ **해싱 및 암호화**
  - SHA256 double hash
  - Merkle root 계산
  - RIPEMD160 (address generation 준비)

- ✅ **직렬화**
  - VarInt 인코딩/디코딩
  - 블록 및 트랜잭션 직렬화/역직렬화

- ✅ **제네시스 블록**
  - 2009년 Bitcoin 제네시스 블록 재현
  - Coinbase 트랜잭션 지원

### 테스트 결과

```
19개 단위 테스트 모두 통과
- Block 직렬화/해시 계산
- Transaction 직렬화/TXID 계산
- Merkle root 계산
- VarInt 인코딩/디코딩
```

## 빌드 및 실행

### 필요 사항

- Rust 1.70 이상
- Cargo

### 빌드

```bash
cargo build
```

### 테스트 실행

```bash
cargo test
```

### 예제 실행

```bash
cargo run
```

## 프로젝트 구조

```
src/
├── lib.rs              # 라이브러리 루트
├── main.rs             # 예제 프로그램
├── core/               # 핵심 데이터 구조 ✓
│   ├── types.rs        # Hash256 등 기본 타입
│   ├── hash.rs         # 해싱 유틸리티
│   ├── serialize.rs    # 직렬화 유틸리티
│   ├── transaction.rs  # 트랜잭션 구조
│   └── block.rs        # 블록 구조
├── consensus/          # 합의 & 검증 (Phase 2)
├── storage/            # 저장소 (Phase 3)
├── network/            # P2P 네트워크 (Phase 4)
└── wallet/             # 지갑 (Phase 5)
```

## 다음 단계: Phase 2 - 검증 로직

### 구현 예정

- [ ] Proof of Work (난이도 고정)
- [ ] Bitcoin Script (P2PKH만)
- [ ] 트랜잭션 검증
- [ ] 블록 검증
- [ ] 채굴 기능 (테스트용)

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
