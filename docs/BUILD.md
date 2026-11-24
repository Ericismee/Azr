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

## Windows 10 compatibility preview

Add `/DWindows10Preview` to the installer build to target Windows 10 22H2 x64 (build 19045). The preview uses the same Azr and guard executables, app ID, install location, and preferences as the standard package. It can update an existing Azr installation. The default installer targets Windows 11 build 22000 or later.

```powershell
ISCC.exe "/DPayloadDir=C:\build\Azr" "/DOutputPath=C:\build\output" packaging\installer.iss
ISCC.exe /DWindows10Preview "/DPayloadDir=C:\build\Azr" "/DOutputPath=C:\build\output" packaging\installer.iss
```

The output files are `Azr-Setup-Windows11-x64.exe` and `Azr-Setup-Windows10-x64.exe`. Keep these filenames on future GitHub releases so the version-independent download links in both READMEs continue to work.

Azr's glass panels can fall back to a color brush when Windows cannot create the blur effect. Window capture can show the system capture border where borderless capture is unavailable. Verify the preview on a Windows 10 22H2 system before listing Windows 10 as a supported operating system. A Windows 11 Sandbox verifies packaging and Windows 11 regression behavior.

**Tiếng Việt.** Thêm `/DWindows10Preview` để tạo bộ cài thử cho Windows 10 22H2 x64, build 19045. Bộ cài thử dùng chung ứng dụng và cấu hình với bản chính, nên có thể cập nhật bản Azr đã cài. Hiệu ứng kính có thể chuyển sang nền màu khi Windows không tạo được hiệu ứng mờ. Cần chạy thử trên Windows 10 22H2 trước khi công bố hỗ trợ chính thức.
