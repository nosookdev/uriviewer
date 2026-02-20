# RustView 빌드 가이드

## 1. Rust 설치 (최초 1회)

```powershell
# PowerShell에서 실행
winget install Rustlang.Rustup
# 또는 브라우저에서 https://rustup.rs 방문 후 rustup-init.exe 실행
```

설치 후 터미널 재시작 필요.

## 2. 의존성 확인

```bash
cargo --version   # cargo 1.xx.x 이상
rustc --version   # rustc 1.xx.x 이상
```

## 3. 빌드

```bash
# 개발용 (빠른 빌드, 디버그 정보 포함)
cargo build

# 실행
cargo run

# 파일 열기
cargo run -- "C:\Users\사용자\Pictures\photo.jpg"

# 릴리즈 빌드 (최적화, 작은 파일)
cargo build --release
# 결과: target\release\rustview.exe
```

## 4. macOS dmg 빌드

```bash
# cargo-bundle 설치 (최초 1회)
cargo install cargo-bundle

# dmg 빌드
cargo bundle --release
# 결과: target/release/bundle/osx/RustView.app
```

## 5. Windows 의존성 (선택)

그래픽 드라이버가 최신이면 별도 설치 없이 동작.
문제 발생 시: Visual C++ Redistributable 설치.

## 지원 포맷

JPG · PNG · GIF · BMP · WebP · TIFF · ICO
