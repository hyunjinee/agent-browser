# Refactoring Tasks

agent-browser 프로젝트의 리팩토링 필요 사항을 정리한 문서입니다.

---

## 1. [Critical] `actions.rs` 파일 분할 (5,481줄)

**파일**: `cli/src/native/actions.rs`

가장 시급한 리팩토링 대상입니다. 단일 파일에 195개의 함수, 120+개의 action handler가 하나의 거대한 `match` 문에 나열되어 있습니다.

### 문제점
- 5,481줄의 단일 파일로 가독성/유지보수성 저하
- `execute_command()` 함수 내 150개 이상의 match arm (630-782줄)
- 관련 없는 기능들이 하나의 파일에 혼재 (navigation, interaction, cookies, state, recording, tabs 등)

### 제안
action handler들을 도메인별 모듈로 분리:

```
cli/src/native/actions/
  mod.rs          -- execute_command(), DaemonState, 공통 유틸리티
  navigation.rs   -- handle_navigate, handle_back, handle_forward, handle_reload, handle_url
  interaction.rs  -- handle_click, handle_fill, handle_type, handle_hover, handle_scroll 등
  query.rs        -- handle_gettext, handle_getattribute, handle_isvisible 등 조회 계열
  tabs.rs         -- handle_tab_list, handle_tab_new, handle_tab_switch, handle_tab_close
  cookies.rs      -- handle_cookies_get/set/clear
  storage.rs      -- handle_storage_get/set/clear
  state.rs        -- handle_state_save/load/list/show/clear/clean/rename
  recording.rs    -- handle_recording_start/stop/restart, handle_har_start/stop, handle_video_start/stop
  media.rs        -- handle_screenshot, handle_snapshot, handle_pdf, handle_screencast_*
  auth.rs         -- handle_auth_save/login/list/delete/show, handle_credentials_*
  advanced.rs     -- handle_route/unroute, handle_dialog, handle_expose, handle_frame 등
```

---

## 2. [Critical] `commands.rs` 파일 분할 (3,963줄)

**파일**: `cli/src/commands.rs`

`parse_command()` 함수가 단일 match 문으로 모든 CLI 명령어를 파싱합니다.

### 문제점
- 3,963줄의 단일 파일
- 반복적인 boilerplate 패턴: selector만 받는 명령어들이 거의 동일한 코드를 반복
  - 예: `hover`, `focus`, `check`, `uncheck` 등이 모두 같은 패턴

### 제안
- `actions.rs`와 동일한 도메인 기준으로 분할
- 반복 패턴을 매크로 또는 헬퍼 함수로 추출:
  ```rust
  // 현재: 각 명령어마다 6-8줄 반복
  "hover" => {
      let sel = rest.first().ok_or_else(|| ParseError::MissingArguments { ... })?;
      Ok(json!({ "id": id, "action": "hover", "selector": sel }))
  }
  // 개선: 단일 selector 명령어 헬퍼
  fn simple_selector_cmd(id: &str, action: &str, rest: &[&str]) -> Result<Value, ParseError>
  ```

---

## 3. [High] `output.rs` 구조 개선 (2,800줄)

**파일**: `cli/src/output.rs`

### 문제점
- help 텍스트, 응답 포맷팅, 컬러 출력이 모두 한 파일에 혼재
- 하드코딩된 help 문자열이 상당 부분 차지

### 제안
- help 텍스트를 별도 모듈 또는 상수 파일로 분리
- 응답 포맷터를 독립 모듈로 추출

---

## 4. [High] `DaemonState` 구조체 비대화

**파일**: `cli/src/native/actions.rs:84-113`

### 문제점
- 30개 이상의 필드를 가진 god object
- 브라우저 상태, 네트워크 상태, 녹화 상태, 인증 상태 등이 하나의 struct에 혼재
- 거의 모든 handler 함수가 `&mut DaemonState`를 통째로 받음

### 제안
상태를 도메인별 하위 구조체로 분리:
```rust
pub struct DaemonState {
    pub browser: BrowserState,      // browser, appium, safari_driver, webdriver_backend, backend_type
    pub network: NetworkState,      // domain_filter, event_tracker, routes, tracked_requests, request_tracking
    pub recording: RecordingConfig, // tracing_state, recording_state, har_recording, har_entries, screencasting
    pub auth: AuthState,            // policy, pending_confirmation, confirm_actions
    pub session: SessionState,      // session_name, session_id, ref_map, active_frame_id
    pub stream: StreamState,        // stream_client, stream_server, inspect_server
}
```

---

## 5. [High] `#[allow(dead_code)]` 남용

**파일**: `cli/src/native/mod.rs`

### 문제점
- `native` 모듈의 **모든** 하위 모듈(21개)에 `#[allow(dead_code)]`가 적용됨
- dead code 경고를 완전히 억제하여 실제 미사용 코드 탐지 불가

### 제안
- `#[allow(dead_code)]`를 모듈 레벨에서 제거
- 실제 미사용 항목을 개별적으로 확인하여 제거하거나, 필요한 곳에만 `#[allow(dead_code)]` 적용
- `pub(crate)` 가시성을 적절히 활용

---

## 6. [Medium] `main.rs` 내 인라인 로직 (880줄)

**파일**: `cli/src/main.rs`

### 문제점
- `run_session()` 함수 내 플랫폼별 PID 확인 로직이 인라인으로 작성 (unsafe 코드 포함)
- `parse_proxy()` 같은 유틸리티 함수가 `main.rs`에 위치
- `main()` 함수의 복잡한 분기 로직

### 제안
- `run_session()`을 `connection.rs` 또는 별도 `session.rs`로 이동
- `parse_proxy()`를 `flags.rs` 또는 `connection.rs`로 이동
- `main()` 내 에러 처리 패턴을 통합

---

## 7. [Medium] handler 함수의 반복적 boilerplate

**파일**: `cli/src/native/actions.rs`

### 문제점
거의 모든 handler가 동일한 패턴을 반복:
```rust
async fn handle_xxx(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
    let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
    let session_id = mgr.active_session_id()?.to_string();
    let selector = cmd.get("selector").and_then(|v| v.as_str())
        .ok_or("Missing 'selector' parameter")?;
    // ...
}
```

### 제안
- 공통 전처리를 매크로 또는 헬퍼로 추출:
  ```rust
  fn require_browser(state: &DaemonState) -> Result<(&BrowserManager, String), String>
  fn require_selector(cmd: &Value) -> Result<&str, String>
  ```
- WebDriver fallback 패턴도 반복적 -- trait 기반 추상화 고려

---

## 8. [Medium] `docs/src/components/docs-chat.tsx` (538줄)

**파일**: `docs/src/components/docs-chat.tsx`

### 문제점
- 채팅 UI의 모든 로직이 단일 컴포넌트에 집중
- 스타일, 상태 관리, 렌더링 로직이 혼재

### 제안
- `ToolCallDisplay`, `ChatMessage`, `ChatInput` 등 하위 컴포넌트로 분리
- 커스텀 hook 추출 (`useChatStorage`, `useResizablePanel` 등)

---

## 9. [Medium] `examples/environments/app/page.tsx` (518줄)

**파일**: `examples/environments/app/page.tsx`

### 문제점
- 전체 페이지 컴포넌트가 단일 파일
- `useIsMobile()`, `useTheme()` 등 재사용 가능한 hook이 인라인

### 제안
- 커스텀 hook들을 `hooks/` 디렉토리로 추출
- 페이지를 논리적 섹션별 컴포넌트로 분할

---

## 10. [Low] `benchmarks/bench.ts` (900줄)

**파일**: `benchmarks/bench.ts`

### 문제점
- 환경 설정, CLI 파싱, 벤치마크 로직, 결과 포맷팅이 한 파일에 집중
- `.env` 수동 파싱 로직 (dotenv 라이브러리 미사용)

### 제안
- 벤치마크 runner, reporter, config를 분리
- dotenv 라이브러리 도입 또는 최소한 env 파싱 유틸리티 분리

---

## 11. [Low] `connection.rs` 내 `#[allow(dead_code)]` 필드

**파일**: `cli/src/connection.rs:16, 31`

### 문제점
- `DaemonOptions` 등 구조체에 `#[allow(dead_code)]` 필드 존재

### 제안
- 실제 사용 여부 확인 후 제거 또는 활용

---

## 12. [Low] 에러 처리 일관성

### 문제점
- action handler들이 `Result<Value, String>`을 사용 -- 구조화된 에러 타입 부재
- `commands.rs`의 `ParseError`는 잘 설계되어 있으나, runtime 에러는 단순 String

### 제안
- `ActionError` enum 도입으로 에러 분류 (BrowserNotLaunched, MissingParameter, CdpError 등)
- 에러 변환 로직 중앙화

---

## 우선순위 요약

| 순위 | 항목 | 영향도 | 난이도 |
|------|------|--------|--------|
| 1 | `actions.rs` 분할 | 높음 | 높음 |
| 2 | `commands.rs` 분할 | 높음 | 중간 |
| 3 | `DaemonState` 분리 | 높음 | 높음 |
| 4 | `#[allow(dead_code)]` 정리 | 중간 | 낮음 |
| 5 | handler boilerplate 제거 | 중간 | 중간 |
| 6 | `output.rs` 분리 | 중간 | 중간 |
| 7 | `main.rs` 정리 | 중간 | 낮음 |
| 8 | 프론트엔드 컴포넌트 분리 | 낮음 | 낮음 |
| 9 | 에러 타입 구조화 | 낮음 | 중간 |
| 10 | 벤치마크 분리 | 낮음 | 낮음 |
