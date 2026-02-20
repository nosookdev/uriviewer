# Glossary — RustView

**Project**: rustview
**Date**: 2026-02-20

---

## Business Terms (프로젝트 내부 용어)

| Term | English | Definition | Global Standard Mapping |
|------|---------|------------|------------------------|
| 뷰어 | Viewer | 이미지를 표시하는 메인 화면 영역 | Image Display Panel |
| 갤러리 | Gallery | 폴더 내 이미지를 썸네일로 보는 뷰 | Thumbnail Grid View |
| 썸네일 | Thumbnail | 갤러리용 소형 미리보기 | Preview Image |
| 뷰 상태 | ViewState | 현재 줌/위치/회전 상태 | Display State |
| 창 맞춤 | Fit Window | 이미지를 창 크기에 맞게 조절 | Zoom to Fit |

## Global Standards (기술 표준 용어)

| Term | Definition | Reference |
|------|------------|-----------|
| EXIF | 디지털 카메라 이미지 메타데이터 포맷 | EXIF 2.32 spec |
| egui | Rust 즉시 모드 GUI 라이브러리 | github.com/emilk/egui |
| eframe | egui 앱 프레임워크 (창 관리) | github.com/emilk/egui |
| TextureHandle | egui의 GPU 텍스처 핸들 | egui docs |
| PathBuf | Rust 파일 경로 타입 | std::path::PathBuf |

## Term Usage Rules

1. 코드에서는 **영어** 사용 (`ViewState`, `Gallery`, `Thumbnail`)
2. UI/문서에서는 **한국어** 사용 가능
3. 타입명은 `PascalCase`, 변수명은 `snake_case`
