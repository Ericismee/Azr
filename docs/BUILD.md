# Build Azr

Requires Windows 11 x64, stable Rust, Visual Studio C++ x64 Build Tools, and the Windows SDK.

```powershell
git clone https://github.com/Ericismee/Azr.git
cd Azr
.\build.ps1
```

The script tests the locked workspace, builds release binaries, and packages a portable ZIP with SHA-256 hashes in `dist`. It does not launch or install Azr.

```powershell
cargo fmt --all -- --check
cargo test --locked --workspace
```

Tiếng Việt: Cần Windows 11 x64, Rust stable, công cụ C++ x64 của Visual Studio Build Tools và Windows SDK. Chạy các lệnh trên bằng PowerShell ở thư mục dự án. Script kiểm thử, build và đóng gói vào `dist`, không tự mở hay cài ứng dụng.
