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

## Installer packaging

The installer recipe is in [packaging/installer.iss](../packaging/installer.iss). It is compiled separately with Inno Setup. Supply `PayloadDir` for the runtime staging folder and `OutputPath` for the generated installer.

The runtime payload contains `azr.exe`, `azr-guard.exe`, the required Visual C++ runtime DLLs, bundled fonts, the brand image, and license files. Exclude source files, debug symbols, and development tools. Test installation, application startup, and uninstallation in Windows Sandbox before publishing.

Tiếng Việt: Công thức đóng gói nằm trong `packaging/installer.iss` và được biên dịch riêng bằng Inno Setup. Gói chạy cần đủ EXE, DLL, font, logo và giấy phép; không kèm mã nguồn hoặc tệp debug. Kiểm tra cài, chạy và gỡ trong Windows Sandbox trước khi phát hành.
