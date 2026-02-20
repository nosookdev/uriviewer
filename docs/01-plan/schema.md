# Schema Definition — RustView

> Phase 1 Deliverable: 이미지 뷰어 데이터 구조 정의

**Project**: rustview
**Date**: 2026-02-20
**Version**: 1.0

---

## 1. Terminology Definition (용어 정의)

| Term (EN) | Term (KO) | Definition | Notes |
|-----------|-----------|------------|-------|
| ImageFile | 이미지 파일 | 디스크에 저장된 이미지 파일 | JPG/PNG 등 포함 |
| Thumbnail | 썸네일 | 갤러리용 소형 미리보기 이미지 | 캐싱 대상 |
| Gallery | 갤러리 | 같은 폴더 내 이미지 목록 뷰 | 썸네일 그리드 |
| Viewport | 뷰포트 | 이미지가 표시되는 화면 영역 | 확대/이동 기준 |
| ZoomLevel | 줌 레벨 | 이미지 표시 배율 (1.0 = 원본) | 0.1 ~ 10.0 |
| Rotation | 회전 | 이미지 회전 각도 (0/90/180/270) | 표시 전용 |
| ViewMode | 뷰 모드 | Single(단일 이미지) / Gallery(갤러리) | - |
| AppConfig | 앱 설정 | 사용자 환경 설정 (TOML 저장) | - |
| EXIF | EXIF | 이미지에 내장된 메타데이터 | 촬영 정보 등 |

---

## 2. Entity List

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `AppState` | 앱 전체 상태 | current_image, view_mode, config |
| `ImageFile` | 로딩된 이미지 | path, size, format, texture_id |
| `ImageMeta` | 이미지 메타데이터 | dimensions, file_size, exif |
| `Thumbnail` | 썸네일 캐시 | path, texture_id, size |
| `Gallery` | 폴더 내 이미지 목록 | folder_path, images, selected_idx |
| `ViewState` | 현재 뷰 상태 | zoom, offset, rotation |
| `AppConfig` | 사용자 설정 | window_size, last_path, theme |

---

## 3. Entity Details

### 3.1 AppState

**Description**: 앱 전체 생명주기를 관리하는 루트 상태

**Attributes**:
| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `current_image` | `Option<ImageFile>` | N | 현재 표시 중인 이미지 |
| `view_mode` | `ViewMode` | Y | Single / Gallery |
| `view_state` | `ViewState` | Y | 줌, 오프셋, 회전 |
| `gallery` | `Option<Gallery>` | N | 갤러리 모드 데이터 |
| `config` | `AppConfig` | Y | 앱 설정 |
| `status_msg` | `Option<String>` | N | 하단 상태바 메시지 |

---

### 3.2 ImageFile

**Description**: 디코딩된 이미지 데이터와 파일 정보

**Attributes**:
| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | `PathBuf` | Y | 파일 절대 경로 |
| `format` | `ImageFormat` | Y | JPG/PNG/GIF/BMP/WebP 등 |
| `texture` | `TextureHandle` | Y | egui 텍스처 핸들 |
| `original_size` | `(u32, u32)` | Y | 원본 해상도 (width, height) |
| `file_size` | `u64` | Y | 파일 크기 (bytes) |
| `meta` | `Option<ImageMeta>` | N | EXIF 등 메타데이터 |

---

### 3.3 ViewState

**Description**: 현재 이미지 뷰의 표시 상태

**Attributes**:
| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `zoom` | `f32` | Y | 배율 (1.0 = 원본, 0.0 = fit) |
| `offset` | `(f32, f32)` | Y | 팬 오프셋 (픽셀) |
| `rotation` | `Rotation` | Y | 0 / 90 / 180 / 270 도 |
| `fit_mode` | `FitMode` | Y | FitWindow / Original / Custom |

---

### 3.4 Gallery

**Description**: 현재 폴더의 이미지 목록

**Attributes**:
| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `folder_path` | `PathBuf` | Y | 폴더 경로 |
| `images` | `Vec<PathBuf>` | Y | 이미지 파일 경로 목록 |
| `selected_idx` | `usize` | Y | 현재 선택된 인덱스 |
| `thumbnails` | `HashMap<PathBuf, Thumbnail>` | Y | 썸네일 캐시 |

---

### 3.5 AppConfig

**Description**: TOML 파일로 저장되는 사용자 설정

**Attributes**:
| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `window_width` | `u32` | Y | 창 너비 |
| `window_height` | `u32` | Y | 창 높이 |
| `last_directory` | `Option<PathBuf>` | N | 마지막 열었던 폴더 |
| `thumbnail_size` | `u32` | Y | 썸네일 크기 px (기본 128) |
| `background_color` | `Color` | Y | 뷰어 배경색 |

---

## 4. Entity Relationship Diagram

```
[AppState] 1 ─── 1 [ViewState]
    │
    ├── 0..1 ── [ImageFile] ─── 0..1 ── [ImageMeta]
    │
    ├── 0..1 ── [Gallery]
    │               │
    │               └── 1 ─── N ── [Thumbnail]
    │
    └── 1 ─── 1 [AppConfig]
```

---

## 5. Enum Definitions

```rust
enum ViewMode {
    Single,   // 단일 이미지 뷰
    Gallery,  // 썸네일 갤러리 뷰
}

enum Rotation {
    R0,    // 0도 (기본)
    R90,   // 90도 시계방향
    R180,  // 180도
    R270,  // 270도 (90도 반시계)
}

enum FitMode {
    FitWindow,  // 창에 맞춤
    Original,   // 원본 크기 (100%)
    Custom,     // 사용자 지정 줌
}

enum ImageFormat {
    Jpeg, Png, Gif, Bmp, WebP, Svg, Ico, Tiff,
}
```

---

## 6. Validation Checklist

- [x] 핵심 엔티티 모두 정의됨
- [x] 용어 명확하고 일관됨
- [x] 엔티티 관계 명확함
- [x] 누락된 속성 없음

---

## 7. Next Steps

Phase 2: Coding Convention 정의 → `CONVENTIONS.md`
