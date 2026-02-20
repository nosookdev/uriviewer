# 이미지 뷰어 (RustView) 계획 문서

> **Summary**: Windows/macOS용 가볍고 빠른 Rust 기반 멀티포맷 이미지 뷰어
>
> **Project**: rustview
> **Version**: 0.1.0
> **Author**: -
> **Date**: 2026-02-20
> **Status**: Draft

---

## 1. Overview

### 1.1 Purpose

기존 이미지 뷰어의 무거움과 느린 로딩 속도를 해결하기 위해, Rust로 작성된 초경량 고성능 이미지 뷰어를 개발한다.

### 1.2 Background

- Windows 기본 사진 앱: 느린 로딩, 무거운 리소스
- 서드파티 뷰어: 광고, 번들 소프트웨어, 큰 설치 용량
- Rust + egui: 단일 바이너리, 빠른 실행, 낮은 메모리

### 1.3 Related Documents

- Schema: `docs/01-plan/schema.md`
- Glossary: `docs/01-plan/glossary.md`

---

## 2. Scope

### 2.1 In Scope

- [x] 기본 이미지 열기/보기 (JPG, PNG, GIF, BMP, WebP, SVG, ICO, TIFF)
- [x] 이전/다음 이미지 탐색 (같은 폴더 기준)
- [x] 확대/축소, 창 맞춤, 원본 크기
- [x] 이미지 회전 (90도 단위)
- [x] 드래그앤드롭 파일 열기
- [x] 썸네일 갤러리 뷰
- [x] 키보드 단축키
- [x] 이미지 정보 패널 (파일명, 크기, 해상도, EXIF)
- [x] Windows exe 빌드
- [x] macOS dmg 빌드

### 2.2 Out of Scope

- 이미지 편집 (크롭, 필터 등)
- RAW 파일 지원 (추후 고려)
- 클라우드 연동
- 슬라이드쇼 (v2 예정)

---

## 3. Requirements

### 3.1 Functional Requirements

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-01 | JPG/PNG/GIF/BMP/WebP/SVG/ICO/TIFF 파일 열기 | High | Pending |
| FR-02 | 파일 탐색기에서 파일 연결 (더블클릭 실행) | High | Pending |
| FR-03 | 드래그앤드롭으로 파일 열기 | High | Pending |
| FR-04 | 이전/다음 이미지 탐색 (같은 폴더) | High | Pending |
| FR-05 | 확대/축소 (마우스 휠, +/- 키) | High | Pending |
| FR-06 | 창 맞춤 / 원본 크기 보기 | High | Pending |
| FR-07 | 이미지 회전 (L/R 키) | Medium | Pending |
| FR-08 | 썸네일 갤러리 뷰 (같은 폴더 이미지) | Medium | Pending |
| FR-09 | 이미지 정보 패널 (파일 크기, 해상도, EXIF) | Medium | Pending |
| FR-10 | 전체화면 모드 (F11 또는 F 키) | Medium | Pending |
| FR-11 | 최근 파일 목록 | Low | Pending |

### 3.2 Non-Functional Requirements

| Category | Criteria | Measurement Method |
|----------|----------|-------------------|
| Performance | 앱 시작 시간 < 500ms | 실측 |
| Performance | 이미지 로딩 < 200ms (10MB 미만) | 실측 |
| Memory | 유휴 메모리 < 50MB | 작업관리자 |
| Binary Size | 실행 파일 < 20MB | 빌드 후 확인 |
| Compatibility | Windows 10+, macOS 12+ | CI 빌드 |

---

## 4. Success Criteria

### 4.1 Definition of Done

- [ ] FR-01 ~ FR-07 구현 완료
- [ ] Windows exe 빌드 성공
- [ ] macOS dmg 빌드 성공
- [ ] 10가지 이상 이미지 포맷 동작 확인

### 4.2 Quality Criteria

- [ ] `cargo clippy` 경고 없음
- [ ] `cargo fmt` 적용
- [ ] 빌드 성공 (debug + release)

---

## 5. Risks and Mitigation

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| HEIC/RAW 포맷 지원 어려움 | Low | High | v1에서 제외, 별도 크레이트 사용 |
| 대용량 이미지 메모리 이슈 | Medium | Medium | 이미지 다운샘플링, 지연 로딩 |
| macOS 코드 서명 | Low | Low | unsigned 빌드로 배포 가능 |

---

## 6. Architecture

### 6.1 Level

**Starter** — 백엔드 없음, 순수 데스크탑 앱

### 6.2 Key Decisions

| Decision | Options | Selected | Rationale |
|----------|---------|----------|-----------|
| GUI Framework | egui, iced, slint | **egui/eframe** | 간단, 빠름, 이미지 뷰어에 최적 |
| Image Decoding | image, fast_image_resize | **image crate** | 포맷 지원 가장 넓음 |
| Build Target | exe, msi, dmg | **exe + dmg** | cross-compile + cargo-bundle |
| Config Storage | JSON, TOML | **TOML** | Rust 생태계 표준 |

### 6.3 Folder Structure

```
rustview/
├── src/
│   ├── main.rs          # 엔트리포인트
│   ├── app.rs           # 앱 상태 & eframe::App 구현
│   ├── viewer.rs        # 이미지 뷰어 UI
│   ├── gallery.rs       # 썸네일 갤러리 UI
│   ├── image_loader.rs  # 이미지 로딩/디코딩
│   ├── file_nav.rs      # 파일 탐색 (이전/다음)
│   └── config.rs        # 앱 설정 저장/불러오기
├── assets/
│   └── icon.png
├── docs/
│   ├── 01-plan/
│   └── 02-design/
├── Cargo.toml
├── CLAUDE.md
└── CONVENTIONS.md
```

---

## 7. Next Steps

1. [x] 계획 문서 작성
2. [ ] Schema/Glossary 작성 (`docs/01-plan/schema.md`)
3. [ ] Convention 정의 (`CONVENTIONS.md`)
4. [ ] Phase 3: UI 목업
5. [ ] Phase 6: 구현
6. [ ] Phase 9: 빌드 및 배포

---

## Version History

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 0.1 | 2026-02-20 | Initial draft | - |
