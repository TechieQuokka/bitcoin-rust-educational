# Architecture Design

## 시스템 아키텍처 개요

교육용 Bitcoin 구현은 모놀리식 구조로 시작하며, 명확한 모듈 분리를 통해 각 컴포넌트의 책임을 구분합니다.

## 핵심 모듈

### 1. Core (핵심 데이터 구조)

블록체인의 기본 데이터 타입과 구조를 정의합니다.

#### Block
- BlockHeader: 버전, 이전 블록 해시, Merkle root, 타임스탬프, 난이도, nonce
- Transaction 리스트
- 블록 해시 계산
- 직렬화/역직렬화

#### Transaction
- TxInput: 이전 트랜잭션 참조, 인덱스, 서명 스크립트
- TxOutput: 금액, 공개키 해시 (P2PKH)
- 트랜잭션 ID 계산 (SHA256 double hash)

#### Script
- P2PKH 스크립트 생성 및 검증
- 필수 opcode 구현 (OP_DUP, OP_HASH160, OP_EQUALVERIFY, OP_CHECKSIG)
- 스크립트 인터프리터

#### Crypto
- 암호화 라이브러리 wrapper
- SHA256, RIPEMD160 해시
- ECDSA 서명/검증 (secp256k1)
- 주소 생성 (Base58Check)

### 2. Consensus (합의 & 검증)

블록체인의 합의 규칙과 검증 로직을 담당합니다.

#### PoW (Proof of Work)
- 난이도 타겟 관리 (고정 난이도)
- 블록 해시 검증
- 채굴 함수 (nonce 탐색)

#### Validation
- 블록 검증 규칙
  - PoW 조건 충족
  - 블록 크기 제한
  - 제네시스 블록 검증
  - Merkle root 일치
- 트랜잭션 검증 규칙
  - 입력 서명 검증
  - 입출력 금액 균형
  - UTXO 존재 확인
  - 이중 지불 방지

#### Chain
- Longest chain rule 구현
- 블록 연결 및 재구성
- 포크 처리
- 블록 높이 관리

### 3. Storage (데이터 저장)

블록체인 데이터와 UTXO를 영구 저장합니다.

#### BlockchainDB
- 블록 저장 및 조회
- 블록 높이별 인덱싱
- 블록 해시로 검색
- 체인 팁 (최신 블록) 관리

#### UTXO Set
- 사용되지 않은 트랜잭션 출력 관리
- 빠른 잔액 조회
- 트랜잭션 검증 시 UTXO 확인
- UTXO 추가/삭제 연산

#### 저장소 기술
- Embedded database: sled
- Key-value 구조
- 트랜잭션 지원 (원자성)

### 4. Network (P2P 네트워킹)

노드 간 통신과 데이터 전파를 처리합니다.

#### Peer Manager
- 피어 연결 관리
- 피어 발견 (초기 시드 노드)
- 연결 유지 및 재연결
- 피어 메타데이터 (버전, 높이)

#### Message Handler
- 프로토콜 메시지 처리
  - Version/Verack: 핸드셰이크
  - GetBlocks/Blocks: 블록 동기화
  - GetData/Tx: 트랜잭션 전파
  - Inv: 새로운 데이터 알림
  - Ping/Pong: 연결 유지

#### Protocol
- 메시지 직렬화/역직렬화
- 메시지 헤더 (magic bytes, 체크섬)
- 네트워크 바이트 순서

#### 네트워킹 기술
- Async runtime: tokio
- TCP 소켓 통신
- 멀티플렉싱 및 동시성

### 5. Wallet (지갑)

키 관리와 트랜잭션 생성을 담당합니다.

#### Keystore
- 개인키/공개키 쌍 생성
- 주소 생성 (P2PKH)
- 키 저장 및 로드
- 여러 키페어 관리

#### Transaction Builder
- UTXO 선택 (코인 선택 알고리즘)
- 트랜잭션 입력/출력 구성
- 서명 스크립트 생성
- 거스름돈 출력 처리
- 트랜잭션 수수료 계산

### 6. CLI (명령줄 인터페이스)

사용자와의 상호작용을 제공합니다.

#### 주요 명령어
- 노드 관리
  - `start`: 노드 시작
  - `stop`: 노드 종료
  - `status`: 노드 상태 조회
- 블록체인 조회
  - `getblock <hash>`: 블록 정보
  - `getblockcount`: 블록 높이
  - `gettx <txid>`: 트랜잭션 정보
- 지갑 기능
  - `getnewaddress`: 새 주소 생성
  - `getbalance`: 잔액 조회
  - `sendtoaddress <address> <amount>`: 송금
  - `listunspent`: UTXO 목록
- 채굴
  - `mine`: 블록 채굴 시작
  - `setmining <on|off>`: 채굴 설정

## 데이터 흐름

### 블록 생성 및 전파
1. 채굴자가 새 블록 생성 (PoW 수행)
2. Consensus 모듈이 블록 검증
3. Storage에 블록 저장
4. Network를 통해 피어에게 전파
5. 피어들이 블록 수신 및 검증
6. 각 노드의 체인에 추가

### 트랜잭션 생성 및 처리
1. 사용자가 CLI를 통해 송금 요청
2. Wallet이 UTXO 선택 및 트랜잭션 생성
3. Core의 Script로 서명 스크립트 작성
4. Consensus가 트랜잭션 검증
5. Network를 통해 피어에게 전파
6. 채굴자가 트랜잭션을 블록에 포함
7. 블록 채굴 후 트랜잭션 확정

### 노드 동기화
1. 새 노드가 네트워크 참여
2. 피어들과 핸드셰이크 (Version/Verack)
3. 블록 높이 비교
4. GetBlocks 메시지로 블록 요청
5. 피어가 Blocks 메시지로 응답
6. 수신한 블록 검증 및 저장
7. 최신 블록까지 동기화 완료

## 동시성 및 안전성

### Async 처리
- tokio runtime으로 비동기 I/O
- 네트워크 요청은 non-blocking
- 여러 피어와 동시 통신

### 스레드 안전성
- Arc/Mutex를 통한 공유 상태 관리
- UTXO set 동시 접근 제어
- 블록체인 상태 일관성 보장

### 에러 처리
- Result 타입으로 명시적 에러 전파
- 네트워크 실패 시 재시도
- 잘못된 데이터 수신 시 피어 차단

## 확장성 고려사항

### 현재 단계
- 단일 프로세스 모놀리식 구조
- 간단한 메모리 풀
- 기본적인 피어 관리

### 향후 개선 가능 영역
- 멀티 크레이트 workspace 분리
- 더 효율적인 UTXO 인덱싱
- 고급 피어 발견 메커니즘
- 메모리 풀 최적화
- 병렬 트랜잭션 검증

## 기술 의존성

### 필수 크레이트
- `tokio`: 비동기 런타임
- `serde`: 직렬화/역직렬화
- `sled`: 임베디드 데이터베이스
- `sha2`: SHA256 해싱
- `secp256k1`: ECDSA 암호화
- `ripemd`: RIPEMD160 해싱

### 추가 유틸리티
- `hex`: 16진수 변환
- `bs58`: Base58 인코딩
- `clap`: CLI 파싱
- `log`, `env_logger`: 로깅
