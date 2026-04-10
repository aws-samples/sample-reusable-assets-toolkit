# RAT CLI 인증 및 SQS 연동 설계

## 개요

rat CLI는 단일 배포가 아닌, 각 팀/조직이 독립적으로 인프라를 배포하는 구조이다.
CLI 사용자는 인프라 소유자와 다른 사람일 수 있으므로, IAM 직접 발급이 아닌 Cognito 기반 인증을 사용한다.

## 아키텍처

```
┌─────────────────────────────────────────────────────┐
│  인프라 소유자가 배포 (CDK)                              │
│                                                     │
│  Cognito User Pool ◄── OIDC Federation (향후 추가)    │
│       │                                             │
│  Cognito Identity Pool (authenticated role)         │
│       │         └─ Policy: sqs:SendMessage          │
│       │                                             │
│  SQS Queue ──► Lambda Consumer ──► RDS (pgvector)   │
└─────────────────────────────────────────────────────┘
        ▲
        │ 임시 AWS credentials
        │
┌───────┴──────────┐
│  CLI 사용자       │
│                  │
│  rat configure   │  ← 소유자가 공유한 endpoint 정보 입력
│  rat login       │  ← 브라우저 OIDC 인증 (PKCE)
│  rat ingest      │  ← SQS 직접 전송
└──────────────────┘
```

## 인증 플로우

### 브라우저 기반 OAuth PKCE

```
rat login
  1. localhost:PORT에 임시 HTTP 서버 기동
  2. PKCE code_verifier + code_challenge 생성
  3. 브라우저 오픈 → Cognito Hosted UI (authorize endpoint)
     https://{domain}/oauth2/authorize
       ?response_type=code
       &client_id={app_client_id}
       &redirect_uri=http://localhost:{PORT}/callback
       &scope=openid+profile+email
       &code_challenge={challenge}
       &code_challenge_method=S256
  4. 사용자가 브라우저에서 인증 (Cognito 자체 or federation IdP)
  5. Cognito redirect → http://localhost:{PORT}/callback?code=AUTH_CODE
  6. 임시 서버가 code 수신, 브라우저에 "인증 완료" 페이지 표시
  7. code + code_verifier로 token endpoint 호출 → 토큰 수신
  8. credentials.json에 저장
  9. 임시 서버 종료
```

### 토큰 → AWS Credentials 변환

```
rat ingest 실행 시:
  1. credentials.json에서 토큰 로드
  2. expires_at 확인
     ├─ 유효 → 그대로 사용
     └─ 만료 → refresh_token으로 token endpoint 호출 → 갱신
  3. id_token을 Cognito Identity Pool에 제출 → 임시 AWS credentials 수신
  4. 해당 credentials로 SQS SendMessage 직접 호출
```

refresh_token은 Cognito 기본 30일 유효.

## OIDC Federation 확장

CLI 코드 변경 없이 Cognito 콘솔에서 IdP 추가만 하면 동작한다.
Hosted UI가 IdP 선택 화면을 자동으로 보여주기 때문.

- Google OIDC
- GitHub (OAuth → Cognito OIDC wrapper)
- SAML (사내 SSO)

## CLI 명령 구조

### `rat configure`

서버 환경 정보를 입력받아 설정 파일에 저장한다.
인프라 소유자가 배포 후 공유하는 값들을 입력한다.

```
rat configure                    # 인터랙티브 설정 (default 프로필)
rat configure --profile staging  # 특정 프로필 설정
rat configure list               # 등록된 프로필 목록
rat configure show               # 현재 활성 설정 출력
```

### `rat login`

Cognito 인증을 수행하고 토큰을 저장한다.

```
rat login                  # 브라우저 열어서 OIDC 인증
rat login --status         # 토큰 유효성 확인
rat logout                 # 토큰 삭제
```

### `rat ingest` (변경)

기존 stdout JSON 출력 대신 SQS로 직접 전송한다.

```
rat ingest ./repo                    # 인증 → SQS 전송
rat ingest ./repo --dry-run          # SQS 미전송, stdout JSON 출력 (기존 동작)
rat ingest ./repo --profile staging  # 특정 프로필 사용
```

설정 파일이 없거나 로그인되지 않은 경우 에러 + 안내 메시지 출력.

## 설정 파일

### `~/.config/rat/config.toml`

```toml
[default]
sqs_queue_url = "https://sqs.ap-northeast-2.amazonaws.com/123456789012/rat-ingest"
aws_region = "ap-northeast-2"
cognito_domain = "my-rat.auth.ap-northeast-2.amazoncognito.com"
cognito_app_client_id = "1a2b3c4d5e6f7g8h9i0j"
cognito_identity_pool_id = "ap-northeast-2:xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"

[profiles.staging]
sqs_queue_url = "https://sqs.ap-northeast-2.amazonaws.com/123456789012/rat-ingest-staging"
aws_region = "ap-northeast-2"
cognito_domain = "my-rat-stg.auth.ap-northeast-2.amazoncognito.com"
cognito_app_client_id = "xxxxxxxxxx"
cognito_identity_pool_id = "ap-northeast-2:yyyyyyyy-yyyy-yyyy-yyyy-yyyyyyyyyyyy"
```

### `~/.config/rat/credentials.json` (0600 권한)

```json
{
  "default": {
    "id_token": "eyJ...",
    "access_token": "eyJ...",
    "refresh_token": "eyJ...",
    "expires_at": 1720000000
  }
}
```

## Cognito 인프라 (CDK AuthStack)

### 리소스

| 리소스 | 설명 |
|--------|------|
| Cognito User Pool | 사용자 관리. Hosted UI 활성화 |
| App Client | Public client (no secret), PKCE 전용, `http://localhost` callback 허용 |
| Identity Pool | User Pool 연동, authenticated role에 SQS 권한 부여 |
| IAM Role (authenticated) | `sqs:SendMessage` on IngestQueue |

### App Client 설정

```
Allowed callback URLs:
  http://localhost:9876/callback

Allowed OAuth flows:
  Authorization code grant

Allowed OAuth scopes:
  openid, profile, email

App client type:
  Public client (no client secret)
```

포트 9876 고정. 충돌 시 에러 메시지 출력 ("포트 9876이 사용 중입니다. 잠시 후 다시 시도하세요").

### SSM 파라미터 (AuthStack에서 저장)

| Key | 값 |
|-----|-----|
| `/idp-code/cognito/domain` | Hosted UI 도메인 |
| `/idp-code/cognito/app-client-id` | App Client ID |
| `/idp-code/cognito/identity-pool-id` | Identity Pool ID |

## 의존성 추가 (rat-cli)

### workspace Cargo.toml

```toml
aws-sdk-sqs = "1"
aws-sdk-cognitoidentity = "1"
toml = "0.8"
dirs = "6"
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
open = "5"
rand = "0.9"
sha2 = "0.10"
base64 = "0.22"
chrono = { version = "0.4", features = ["serde"] }
```

### rat-cli/Cargo.toml

```toml
aws-config.workspace = true
aws-sdk-sqs.workspace = true
aws-sdk-cognitoidentity.workspace = true
toml.workspace = true
dirs.workspace = true
reqwest.workspace = true
open.workspace = true
rand.workspace = true
sha2.workspace = true
base64.workspace = true
chrono.workspace = true
```

## 소스 구조 (rat-cli)

```
rat-cli/src/
├── config.rs     # 설정 파일 읽기/쓰기, Profile 구조체
├── auth.rs       # PKCE 플로우, 토큰 저장/갱신, Identity Pool credentials
├── sqs.rs        # SQS 클라이언트 초기화 + send_message
├── main.rs       # configure, login, logout 서브커맨드 추가 + ingest SQS 연동
├── message.rs    # (기존) FileMessage 구조체
├── chunk/        # (기존) tree-sitter 청킹
├── git.rs        # (기존) git 연동
└── ratignore.rs  # (기존) .ratignore
```

### 핵심 타입

```rust
// config.rs
#[derive(Serialize, Deserialize)]
pub struct RatConfig {
    pub default: Profile,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

#[derive(Serialize, Deserialize)]
pub struct Profile {
    pub sqs_queue_url: String,
    pub aws_region: String,
    pub cognito_domain: String,
    pub cognito_app_client_id: String,
    pub cognito_identity_pool_id: String,
}

// auth.rs
#[derive(Serialize, Deserialize)]
pub struct TokenSet {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}
```

## SQS 전송 시 고려사항

- SQS 메시지 최대 크기 256KB
- 큰 파일은 향후 S3에 올리고 reference만 전송하는 패턴 필요
- 현재는 `send_message` 단건 호출, 추후 `send_message_batch` (10건 단위) 최적화
