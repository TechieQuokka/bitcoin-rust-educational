# Bitcoin Educational Implementation - Project Overview

## 프로젝트 목표

2009년 사토시 나카모토의 Bitcoin v0.1.0을 Rust로 재구현하여 블록체인 핵심 개념을 학습하는 교육용 프로젝트

## 핵심 방향성

### 구현 범위
- **풀 노드 구현** + CLI 인터페이스
- P2P 네트워크, 블록 생성/검증, 트랜잭션 처리, 지갑 기능 포함

### 충실도 전략
- **핵심 로직만 충실하게 (Option B)**
  - 합의 알고리즘, 트랜잭션 검증 등 코어 로직은 2009년 방식 그대로
  - 명백한 버그나 보안 취약점은 수정
  - 데이터 구조와 구현은 현대적 Rust 방식 적용

### 네트워크 호환성
- **독립적인 교육용 체인**
- 실제 Bitcoin 메인넷/테스트넷과 호환 불필요
- 프로토콜 호환성 걱정 없이 학습에 집중

## 기술 스택 결정

### 직접 구현 (학습 중심)
- 블록/트랜잭션 데이터 구조
- 검증 로직 (PoW, 서명 검증)
- Merkle tree (기존 구현 통합)
- 합의 알고리즘
- UTXO 모델

### 라이브러리 사용 (효율성/안전성)
- **암호화**: SHA256 (`sha2`), ECDSA (`secp256k1`), RIPEMD160 (`ripemd`)
- **네트워킹**: `tokio` async runtime
- **데이터베이스**: `sled` embedded DB
- **직렬화**: `serde`

## 아키텍처 구조

### 프로젝트 구성
- **모놀리식 구조**로 시작
- 필요시 나중에 workspace로 분리 가능

### 모듈 구조
```
bitcoin-rust/
├── core/           # 핵심 데이터 구조
│   ├── block
│   ├── transaction
│   ├── script
│   └── crypto
│
├── consensus/      # 합의 & 검증
│   ├── pow
│   ├── validation
│   └── chain
│
├── storage/        # 데이터 저장
│   ├── blockchain_db
│   └── utxo_set
│
├── network/        # P2P 네트워킹
│   ├── peer_manager
│   ├── message_handler
│   └── protocol
│
├── wallet/         # 지갑 기능
│   ├── keystore
│   └── transaction_builder
│
└── cli/            # CLI 인터페이스
```

## 기능 범위 정의

### Bitcoin Script
- **간소화된 구현**: P2PKH (Pay-to-Public-Key-Hash)만 지원
- OP_CHECKSIG, OP_DUP 등 필수 opcode만 구현
- 복잡한 스크립트는 제외

### 채굴 (Mining)
- **테스트용 간단 버전**
- CPU 채굴 지원
- **난이도 고정** (난이도 조정 알고리즘 제외)
- 학습 및 테스트 목적

### 지갑
- **기본 기능만 구현**
  - 키페어 생성/관리
  - 잔액 조회 (UTXO 기반)
  - 트랜잭션 생성 (코인 보내기)
- HD Wallet 미지원 (BIP32는 2009년 이후)

### 인터페이스
- **CLI만 구현**
- JSON-RPC 인터페이스 제외
- 명령줄 기반 상호작용

## 개발 단계

### 개발 접근 방식
- **Bottom-up 방식**: 기초 데이터 구조부터 시작하여 점진적으로 확장

### Phase 1: 기초 데이터 구조
- Block, BlockHeader 정의
- Transaction, TxInput, TxOutput 구조
- 기본 직렬화/역직렬화
- **목표**: 제네시스 블록 생성, 블록 해시 계산

### Phase 2: 검증 로직
- PoW 검증 (난이도 고정)
- P2PKH 스크립트 검증 (ECDSA 서명)
- 기본 트랜잭션 검증 규칙
- **목표**: 수동으로 만든 블록/트랜잭션 검증 가능

### Phase 3: 블록체인 & 저장소
- Chain 관리 (longest chain rule)
- UTXO set 관리
- 데이터베이스 연동 (sled)
- **목표**: 여러 블록을 체인으로 연결 및 저장

### Phase 4: P2P 네트워크
- P2P 연결 (tokio TCP)
- 블록/트랜잭션 전파
- 피어 관리 및 발견
- **목표**: 여러 노드가 서로 블록 공유

### Phase 5: 지갑 & CLI
- 키페어 생성/관리
- 트랜잭션 빌더
- CLI 명령어 인터페이스
- **목표**: 실제 사용 가능한 CLI 도구

## 학습 목표

### 핵심 개념 이해
- UTXO 모델과 트랜잭션 구조
- Proof of Work 합의 알고리즘
- P2P 네트워크와 블록 전파
- 암호화 서명과 검증

### 실용적 경험
- Rust 비동기 프로그래밍 (tokio)
- 데이터 직렬화 및 저장
- 분산 시스템 설계
- CLI 도구 개발

## 성공 기준

### 기능적 목표
- 독립적인 P2P 네트워크에서 여러 노드 실행
- 노드 간 블록과 트랜잭션 전파
- CLI를 통한 코인 송수신
- 채굴을 통한 새로운 블록 생성

### 교육적 목표
- 2009년 Bitcoin 핵심 로직 완전 이해
- 블록체인 기술의 근본 원리 체득
- 분산 합의 메커니즘 실제 구현 경험
