# Refactoring Tasks

agent-browser 코드베이스에 대한 종합적인 리팩토링 분석 결과입니다.

---

## 1. 대형 파일 분할 (Critical)

### 1.1 `actions.rs` 분할 (6,659줄)
- **현재 문제**: 60개 이상의 `handle_*` 핸들러, 커맨드 라우팅, CDP 인터랙션, 비즈니스 로직이 하나의 파일에 혼재
- **작업**:
  - 커맨드 라우팅 로직 → `dispatch.rs`
  - 네비게이션 핸들러 (open, goto, navigate, back, forward, reload) → `handlers/navigation.rs`
  - 인터랙션 핸들러 (click, fill, select, type, press, scroll, hover, focus) → `handlers/interaction.rs`
  - 인스펙션 핸들러 (snapshot, get, find) → `handlers/inspection.rs`
  - 미디어 핸들러 (screenshot, video) → `handlers/media.rs`
  - 상태 관리 핸들러 (state save/load, auth, session) → `handlers/session.rs`
  - `DaemonState` 구조체 (20개 이상 필드) → `daemon_state.rs`로 분리

### 1.2 `commands.rs` 분할 (4,061줄)
- **현재 문제**: 모든 커맨드 파싱 로직이 단일 파일에 집중
- **작업**:
  - 커맨드 카테고리별 서브모듈로 분리
  - 공통 파싱 유틸리티 추출

### 1.3 `output.rs` 분할 (2,861줄)
- **현재 문제**: JSON 포맷팅, 에러 메시지, 스냅샷, 헤더 처리 혼재
- **작업**:
  - JSON 포맷팅 → `output/json.rs`
  - 에러 포맷팅 → `output/errors.rs`
  - 스냅샷 출력 → `output/snapshot.rs`

### 1.4 `e2e_tests.rs` 분할 (3,203줄)
- **작업**: 기능별 테스트 파일로 분리 (core, interaction, advanced)

### 1.5 `flags.rs` 분할 (1,368줄)
- **작업**: 설정 로딩, 플래그 파싱, 유효성 검사를 별도 모듈로 분리

---

## 2. 코드 중복 제거 (High)

### 2.1 핸들러 함수 패턴 통합
- **현재 문제**: 30개 이상의 `handle_*` 함수가 거의 동일한 시그니처와 보일러플레이트 반복
  ```rust
  async fn handle_click(cmd: &Value, state: &mut DaemonState) -> Result<Value, String>
  async fn handle_dblclick(cmd: &Value, state: &mut DaemonState) -> Result<Value, String>
  async fn handle_fill(cmd: &Value, state: &mut DaemonState) -> Result<Value, String>
  ```
- **작업**: 공통 핸들러 trait 또는 매크로를 도입하여 파라미터 추출 → 브라우저 검증 → 세션 ID 획득 → 액션 실행 → 결과 반환 패턴 통합

### 2.2 Arc/RwLock 클론 패턴 정리 (`stream.rs`)
- **현재 문제**: `_clone`, `_bg` 접미사의 변수 클론이 반복
  ```rust
  frame_tx_clone, client_count_clone, client_slot_clone, notify_clone, screencasting_clone, cdp_session_clone
  ```
- **작업**: 헬퍼 함수로 Arc/RwLock 그룹 클론 추출

### 2.3 Config 병합 로직 중복
- **현재 문제**: `Config::merge()`에서 37개 필드를 수동으로 하나씩 병합
- **작업**: 매크로 또는 derive 매크로를 사용하여 자동화

---

## 3. 에러 처리 개선 (High)

### 3.1 구조화된 에러 타입 도입
- **현재 문제**: 전체 코드베이스에서 `Result<T, String>` 사용, 에러 컨텍스트 손실
- **작업**:
  - 모듈별 에러 enum 정의 (예: `ActionError`, `BrowserError`, `CdpError`)
  - `thiserror` 크레이트 도입 검토
  - `commands.rs`의 `ParseError` 패턴을 전체 코드베이스로 확대

### 3.2 `unwrap()` 호출 제거 (328개)
- **주요 위치**:
  - `auth.rs` 78줄: hex 파싱에서 `unwrap()` → 에러 전파로 변경
  - `auth.rs` 434-510줄: 암호화 로직에서 다수의 `unwrap()` → `?` 연산자로 변경
  - `tracing.rs` 294-306줄: 프로파일러 데이터 직렬화
  - `state.rs` 515-601줄: 상태 직렬화
  - `cdp/chrome.rs` 547줄: `addr.parse().unwrap()` → 에러 전파
  - 테스트 코드의 `unwrap()` → `expect("설명")`로 변경

### 3.3 무시되는 에러 처리
- **현재 문제**: `let _ = ...` 패턴으로 에러 무시
  - `stream.rs` 168, 186, 204, 213줄: broadcast 전송 실패 무시
  - `daemon.rs`: 파일 쓰기 실패 무시
- **작업**: 로깅 추가 또는 명시적 에러 처리

### 3.4 AI-friendly 에러 매핑 일관성
- **현재 문제**: `browser.rs`에 `to_ai_friendly_error()` 존재하지만 일관되게 적용되지 않음
- **작업**: 모든 사용자 facing 에러에 대해 일관된 AI-friendly 에러 매핑 적용

---

## 4. 타입 안전성 강화 (High)

### 4.1 커맨드 파라미터 타입 구조체 도입
- **현재 문제**: 모든 핸들러가 `&Value` (untyped JSON)를 받아 런타임에 파싱
  ```rust
  let new_tab = cmd.get("newTab").and_then(|v| v.as_bool()).unwrap_or(false);
  let click_count = cmd.get("clickCount").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
  ```
- **작업**: serde를 활용한 타입 구조체 정의로 컴파일 타임 검증
  ```rust
  #[derive(Deserialize)]
  struct ClickCommand {
      selector: String,
      #[serde(default)]
      new_tab: bool,
      #[serde(default = "default_click_count")]
      click_count: i32,
  }
  ```

### 4.2 액션 이름 문자열 → Enum 변환
- **현재 문제**: `policy.rs`에서 `check(&self, action: &str)` 사용
- **작업**: `ActionKind` enum 도입으로 유효하지 않은 액션 이름을 컴파일 타임에 방지

### 4.3 Option/Result 체인 간소화
- **현재 문제**: `.get().and_then().and_then().ok_or()` 패턴 반복
- **작업**: 헬퍼 함수 추출로 가독성 개선

---

## 5. 관심사 분리 (Medium)

### 5.1 `DaemonState` 분리
- **현재 문제**: 20개 이상 필드가 단일 구조체에 집중 (브라우저, WebDriver, Appium, Safari, 이벤트, 라우팅, HAR, 스트림 등)
- **작업**:
  - `BrowserState` - 브라우저 프로세스 관리
  - `NetworkState` - 네트워크 인터셉션, HAR
  - `EventState` - 이벤트 추적
  - `InspectState` - 디버그 인스펙션

### 5.2 `execute_command()` 분리
- **현재 문제**: 이벤트 드레이닝, 정책 체크, 액션 라우팅, 응답 포맷팅 혼재
- **작업**: 파이프라인 패턴 도입 (Protocol → Parsing → Business Logic → Output)

### 5.3 CDP 직접 호출 정리
- **현재 문제**: WebDriver trait(`BrowserBackend`)을 우회하여 `CdpClient` 직접 호출
- **작업**: 브라우저 백엔드 추상화 레이어를 통한 일관된 접근

---

## 6. 테스트 커버리지 개선 (Medium-High)

### 6.1 단위 테스트 부재 해결
- **현재 문제**: 전통적인 `#[test]` 단위 테스트 0개, e2e 테스트 42개는 모두 `#[ignore]`
- **테스트가 필요한 모듈**:
  - `commands.rs` - 커맨드 파싱 로직 (4,061줄, 테스트 0개)
  - `connection.rs` - IPC/소켓 통신 (714줄, 테스트 0개)
  - `flags.rs` - 플래그 파싱/설정 (1,368줄, 테스트 미미)
  - `output.rs` - 출력 포맷팅 (2,861줄, 테스트 0개)
  - WebDriver 구현체 (iOS, Safari, Appium) - 테스트 0개

### 6.2 테스트 코드 분리
- **현재 문제**: `#[cfg(test)]` 블록이 소스 파일 내부에 산재
- **작업**: 별도 테스트 모듈/파일로 정리

---

## 7. 하드코딩 값 상수화 (Medium)

### 7.1 매직 넘버 제거
| 위치 | 값 | 설명 |
|------|-----|------|
| `stream.rs:102` | `64` | broadcast 채널 사이즈 |
| `recording.rs:12-13` | FPS/간격 값 | 캡처 인터벌 |
| 여러 곳 | CDP 이벤트 문자열 | `"Input.dispatchMouseEvent"` 등 |
| `stream.rs:174-177` | JSON 키 | `"offsetTop"`, `"pageScaleFactor"` |

### 7.2 설정 검증 강화
- `executable_path` 존재 여부 미검증
- `profile` 경로 미검증
- 숫자형 문자열 타입 검증 부재

---

## 8. 네이밍 일관성 (Low)

### 8.1 파라미터 네이밍 통일
- `cmd` vs `command`, `mgr` vs `manager` vs `browser` → 일관된 명명 규칙 적용

### 8.2 짧은 변수명 개선
- `stream.rs`: `tid` → `target_id`, `chp` → `chrome_host_port`
- `snapshot.rs`: `e1`, `e2` → 의미 있는 이름으로 변경

### 8.3 클론 변수 접미사 정리
- `_clone`, `_bg`, `_for_send` 접미사 대신 스코프 활용

---

## 9. 설계 패턴 개선 (Low)

### 9.1 Builder 패턴 도입
- `DaemonState::new()`가 환경변수에서 직접 초기화 → `DaemonStateBuilder` 도입

### 9.2 Trait Object 활용
- WebDriver 백엔드: 런타임 타입 체크 대신 trait object 활용
- `webdriver_backend: Option<WebDriverBackend>` → `Box<dyn BrowserBackend>`

### 9.3 CdpClient 타입 단순화
- **현재 문제**: 복잡한 중첩 제네릭
  ```rust
  ws_tx: Arc<Mutex<...>>
  pending: Arc<Mutex<HashMap<u64, oneshot::Sender<CdpMessage>>>>
  ```
- **작업**: 타입 앨리어스 도입으로 가독성 개선

---

## 우선순위 요약

| 우선순위 | 카테고리 | 예상 영향 |
|----------|---------|----------|
| **P0 - Critical** | actions.rs 분할 (6,659줄) | 유지보수성 대폭 향상 |
| **P0 - Critical** | unwrap() 328개 제거 | 런타임 패닉 방지 |
| **P1 - High** | 핸들러 중복 제거 | 코드량 30%+ 감소 |
| **P1 - High** | 구조화된 에러 타입 도입 | 디버깅/에러 처리 개선 |
| **P1 - High** | 커맨드 타입 구조체 도입 | 컴파일 타임 안전성 |
| **P2 - Medium** | DaemonState 분리 | 관심사 분리 |
| **P2 - Medium** | 테스트 커버리지 확대 | 코드 신뢰성 |
| **P2 - Medium** | 하드코딩 값 상수화 | 설정 가능성 |
| **P3 - Low** | 네이밍 일관성 | 가독성 |
| **P3 - Low** | 설계 패턴 개선 | 확장성 |
