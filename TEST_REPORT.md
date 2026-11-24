# Kiểm thử Azr 0.1.6 trên Windows 11

Ngày 24/11/2026. Bản kiểm thử dùng hồ sơ Azr tạm, tài khoản Windows có mức toàn vẹn **Medium** trên máy chính, và Windows Sandbox tắt mạng để thử trình cài đặt và khởi động lại Explorer.

## Môi trường

- **Máy chính:** Windows 11 Pro build 26300, Ryzen 7 7840H (16 luồng), RAM khả dụng cho hệ điều hành khoảng 27,7 GiB, Radeon 780M và GeForce RTX 3050 Laptop.
- **Hai màn hình:** 1920 × 1080 ở 144 Hz và 2880 × 1800 ở 90 Hz theo thông tin bộ điều khiển đồ họa. Tần số trình bày trong từng ca có thể khác thông số này.
- **Windows Sandbox:** Windows 11 Enterprise build 26100, DPI thực của cửa sổ menu bar trong bài thử là 96 (100%). Sandbox có vGPU.
- **Build:** Rust 1.98.1, MSVC x64, `cargo build --locked --release --workspace`, Control Flow Guard, LTO, không kèm PDB trong gói. Installer Inno Setup chạy ở phạm vi người dùng hiện tại. Gói portable chứa hai EXE, hai DLL Visual C++, font, logo và thông báo giấy phép.

## Phạm vi theo mã nguồn

Đối chiếu README tiếng Anh và tiếng Việt, các manifest Cargo và các module của workspace.

| Nhóm | Module và chức năng đã nhận diện |
| --- | --- |
| Ứng dụng và khôi phục shell | `app/main.rs`, `mainwin.rs`, `single_instance.rs`, `guard`, `platform/shell`. Khởi động, biểu tượng tray, taskbar, tự khởi động và khôi phục Windows shell. |
| Dock | `app/dock`, `core/dock`. Ghim, sắp xếp, kéo thả, biểu tượng đang chạy, mở/thu nhỏ/đóng cửa sổ, preview, tooltip, tải xuống, thùng rác, ẩn tự động và chế độ toàn màn hình. |
| Tìm kiếm và Launchpad | `app/spotlight.rs`, `launchpad.rs`, `core/search`, `platform/filesearch`. Ứng dụng, tệp, phân trang, lối tắt và thao tác tìm kiếm. |
| Dynamic Island | `app/notch`, `core/island`, `platform/media`. Đồng hồ, pin, media, timer, lịch, tải xuống, File Shelf và các nội dung theo cấu hình. |
| Menu và bảng nổi | `menubar.rs`, `controlcenter.rs`, `netmenu.rs`, `popup.rs`, `hud.rs`, `power.rs`. Menu ứng dụng, âm thanh, màn hình, kết nối, nguồn và menu theo ngữ cảnh. |
| Desktop và cửa sổ | `mission.rs`, `stage.rs`, `switcher.rs`, `fx.rs`, `widgets.rs`, `hotcorners.rs`. Tổng quan, đổi cửa sổ, desktop, hiệu ứng thu nhỏ, widget và góc màn hình. |
| Cài đặt và hiển thị | `settings_win.rs`, `prefs.rs`, `shortcuts.rs`, `core/settings.rs`, `display.rs`, `ui/glass`. Lưu tùy chọn, địa phương hóa, phím tắt, nền mờ/khúc xạ, tỷ lệ UI, DPI và nhiều màn hình. |

## Checklist thực thi

| Nhóm kiểm tra | Trạng thái | Phạm vi xác nhận |
| --- | --- | --- |
| Build release và kiểm thử Rust | Pass | 273 đạt, 0 lỗi, 2 bài được đánh dấu bỏ qua. Hai bài là xuất QR cho bộ giải mã và kiểm tra clipboard Windows có ghi đè nội dung thật. |
| Cài mới, mở, thoát, mở lại | Pass | Sandbox cài thành công với bộ thư viện đi kèm. Ba vòng khởi động và thoát sạch, kiểm tra hash EXE đã cài so với bản portable. |
| Mở thêm instance | Pass sau sửa | Lần gọi bình thường thoát. Lần gọi `--open-settings` hiện Cài đặt của tiến trình cũ. Lối tắt Start Menu được cấu hình truyền đúng tham số. |
| Dock | Một phần | Kiểm tra tương tác hover/leave lặp 30 vòng và vị trí so với thanh tác vụ. Kiểm tra hồi quy mở → thu nhỏ → khôi phục cùng cửa sổ Explorer. Cần thẩm định đầy đủ kéo ghim/thả file, độ mượt phóng to và preview với nhiều loại ứng dụng. |
| Tìm kiếm nhanh và Launchpad | Một phần | Kiểm tra vòng đời cửa sổ lặp, thời gian hiện và đóng. Chưa kiểm đủ tìm kiếm tệp, phân trang khi nhiều ứng dụng và khoảng cách giữa các khung hình zoom/fade. |
| Dynamic Island | Một phần | Kiểm tra mở rộng và thu gọn 24 vòng khi dùng nguồn media mô phỏng. Chưa phủ toàn bộ nguồn media thật, thông báo, timer/lịch/tải tệp và click ngoài qua chuột thật. |
| Bảng điều khiển, popup, widget | Một phần | Bảng điều khiển lặp 24 vòng. Các ca thiết bị thật, chỉnh kích thước widget và tương tác từng nút hệ thống cần kiểm tra thêm. |
| Glass, blur, theme và DPI | Một phần | Đã quan sát trực tiếp Dock kính trên hai nền thử sáng/tối và khi Launchpad chồng lên Cài đặt trong Sandbox. Cần đo viền và chuyển động theo thời gian, thử toàn bộ vật liệu và DPI thực 125%, 150%, 200%. |
| Cài đặt và phím tắt | Một phần | Đã đổi tiếng Anh sang tiếng Việt và dùng chuột ghi Ctrl+Shift+K để mở Tìm kiếm nhanh trong Sandbox. Kiểm tra Esc đóng tìm kiếm và lần gọi thứ hai mở Cài đặt. Chưa thao tác từng tùy chọn trong UI. |
| Explorer | Pass | Khởi động lại Explorer thật trong Sandbox. Azr nhận `TaskbarCreated`, Dock vẫn sẵn sàng và ứng dụng tiếp tục chạy. |
| Sleep/wake, fullscreen và phần cứng | Một phần | Các kiểm tra dựa trên thông điệp và cửa sổ thử chỉ xác nhận đường xử lý tương ứng. Sleep/wake thật, game exclusive fullscreen và tháo màn hình vật lý vẫn thiếu. |
| Uninstall | Pass | Uninstaller trả mã 0 và xóa EXE khỏi thư mục cài đặt trong Sandbox. |

Build release hoàn thành. Có 37 cảnh báo Rust, gồm 3 từ `azr-core` và 34 từ `azr-app`. Chủ yếu là import, biến, trường, hàm chưa dùng và giá trị gán bị ghi đè. Không có lỗi biên dịch hoặc liên kết. Không quan sát thấy thiếu DLL, panic hay crash trong các ca release đã chạy.

## Lỗi theo mức độ ưu tiên

| Mức | Tái hiện | Mong đợi / thực tế | Module và nguyên nhân | Xử lý |
| --- | --- | --- | --- | --- |
| High | Giữ thanh tác vụ Windows hiện. Chọn Dock cố định, mở Azr ở 1920 × 1080. | Dock nằm phía trên thanh tác vụ. Trước sửa đáy Dock là y=1080 và thanh tác vụ bắt đầu ở y=1032. | `app/dock/mod.rs`, `platform/shell/taskbar.rs`. Điểm neo Dock luôn dùng đáy màn hình, kể cả khi thanh tác vụ chiếm mép dưới. | Đã sửa và thử lại. Đáy Dock bằng y=1032 trên máy chính, bằng y=775 trong Sandbox, vừa sát cạnh trên thanh tác vụ ở cả hai ca. |
| High | Mở File Explorer bằng biểu tượng Dock. Khi Explorer đang ở trước, bấm lại cùng biểu tượng. | Cửa sổ thu nhỏ và lần bấm kế tiếp khôi phục nó. Bản trước sửa đóng cửa sổ Explorer. | `app/dock/mod.rs`. Nhánh cửa sổ đang ở trước gửi yêu cầu đóng thay vì dùng đường kích hoạt và thu nhỏ sẵn có. | Đã sửa và thử lại trong Sandbox. Cả lần thu nhỏ và khôi phục giữ nguyên mã cửa sổ Win32. |
| Medium | Để Azr chạy, đóng Cài đặt, chạy bản thứ hai với `--open-settings`. | Hiện Cài đặt từ tiến trình sẵn có. Trước sửa tiến trình thứ hai thoát và bản đang chạy chỉ ghi log. | `app/single_instance.rs`, `app/main.rs`, `packaging/installer.iss`. Tín hiệu truyền tới vòng lặp mà không mở cửa sổ. | Đã sửa và thử lại. Sự kiện được xử lý trên luồng UI của tiến trình chính. Lối tắt Start Menu và desktop truyền tham số. |
| Medium, hiệu năng | Khởi động Azr mới rồi mở lần đầu Launchpad hoặc Bảng điều khiển nhanh. | Thời gian mở lần đầu nên gần các lần mở tiếp theo. Bảng điều khiển nhanh có lần mở 206,8 ms trong Sandbox, trung vị 46,2 ms. | `app/controlcenter.rs`, `app/spotlight.rs`, `app/launchpad.rs`. Khởi tạo giao diện, cập nhật dữ liệu và tải biểu tượng là các ứng viên cần tách bằng trace. | Đo trace riêng từng giai đoạn của lần mở đầu và thử làm nóng các dữ liệu nhẹ. Chưa xác nhận nguyên nhân của chênh lệch này. |

## Số đo hiệu năng

Các bài stress gửi thông điệp tới cửa sổ Azr đang chạy trong phiên Windows. Thời gian popup tính từ lúc gửi lệnh tới khi HWND hiện và DWM flush xong. Với Dynamic Island, đo lúc vùng tương tác mở rộng. Các chỉ số này đo độ phản hồi của một phần đường mở UI.

Bảng ghi kết quả đợt stress trước thay đổi thao tác bấm Dock. Trên gói cuối, Sandbox chạy lại 24 vòng mỗi bề mặt. Trung vị lần lượt là 48,9 ms cho Tìm kiếm nhanh, 72,4 ms cho Launchpad, 48,6 ms cho Bảng điều khiển nhanh và 15,3 ms cho vùng tương tác Dynamic Island.

| Thành phần | Số vòng mỗi môi trường | Máy chính trung vị / max | Sandbox trung vị / max |
| --- | ---: | ---: | ---: |
| Tìm kiếm nhanh | 24 | 53,6 / 75,9 ms | 49,7 / 65,3 ms |
| Launchpad | 24 | 58,9 / 124,0 ms | 61,1 / 130,9 ms |
| Bảng điều khiển nhanh | 24 | 46,8 / 120,2 ms | 46,2 / 206,8 ms |
| Dynamic Island | 24 | 15,4 / 27,3 ms | 15,6 / 37,9 ms |
| Dock | 30 | Qua lượt hover/leave | Qua lượt hover/leave |

| Tài nguyên tiến trình `azr.exe` | Máy chính, trước → ngay sau → ổn định 15 giây | Sandbox, trước → ngay sau → ổn định 15 giây |
| --- | --- | --- |
| CPU tích lũy | 1,41 → 9,08 → 9,25 giây | 0,61 → 6,59 → 6,72 giây |
| Working set | 60,84 → 20,01 → 20,29 MiB | 52,22 → 16,45 → 17,25 MiB |
| Private bytes | 17,61 → 23,57 → 24,38 MiB | 13,71 → 19,40 → 19,40 MiB |
| Handles | 661 → 764 → 762 | 588 → 675 → 675 |
| Đối tượng GDI | 44 → 47 → 47 | 44 → 47 → 47 |
| Đối tượng USER | 74 → 90 → 88 | 54 → 68 → 66 |
| Tổng counter GPU Engine tại thời điểm đọc | 0% cả ba mẫu | 0% cả ba mẫu |

Private bytes và handles tăng sau khi đã mở nhiều UI lần đầu. Một lượt stress và khoảng chờ 15 giây chưa đủ phân biệt cache giữ lại với rò rỉ. Cần lặp nhiều khối stress liên tiếp và theo dõi xu hướng qua từng khối. Counter GPU tại vài thời điểm không đo đỉnh trong animation và không phân bổ được chi phí compositor DWM cho Azr.

### Theo dõi dài hơn trên máy chính

- Bản cục bộ 0.1.5 làm mốc, khoảng 10 phút hoạt động nền. CPU của `azr.exe` tăng 4,95 giây, tương đương khoảng 0,83% của một luồng logic. Working set 30,23 → 34,93 MiB, private bytes 20,55 → 21,73 MiB. Counter GPU của tiến trình đọc 0% ở các mẫu.
- Bản 0.1.6 trước thay đổi lần bấm Dock cuối ghi được khoảng 7 phút trước khi phiên đo bị ngắt. CPU của `azr.exe` tăng 4,13 giây, khoảng 0,97% của một luồng. Working set 69,51 → 29,96 MiB, private bytes 19,98 → 20,56 MiB. Hai phiên có mức hoạt động desktop khác nhau nên chưa thể dùng số chênh lệch để kết luận hồi quy.

### Animation và hiệu ứng còn cần đo

- Lần mở đầu Launchpad và Bảng điều khiển nhanh có độ trễ cao hơn rõ rệt. Cần đo riêng thời gian dựng giao diện, tải biểu tượng và lấy trạng thái hệ thống.
- Qua các vòng mở/đóng, cửa sổ trở về trạng thái mong đợi. Chưa có trace trình bày theo khung hình cho Dock, Launchpad, Dynamic Island và hiệu ứng glass. Chưa kết luận đạt 60 FPS, số khung hình rơi hoặc độ đều trên các màn hình 90/144 Hz.

## Thay đổi trong đợt kiểm thử

- Sửa đường mở Cài đặt khi Azr đã chạy và cập nhật lối tắt cài đặt.
- Đặt Dock phía trên thanh tác vụ Windows đang hiện. Giữ vùng kích hoạt Smart Dock ở sát mép vật lý và thêm bài kiểm thử hình học cho màn hình có tọa độ âm.
- Dùng cùng đường kích hoạt và thu nhỏ khi bấm biểu tượng ứng dụng đang chạy trên Dock. Với một nhóm cửa sổ, ưu tiên cửa sổ đang ở trước hoặc gần đây nhất. Đóng cửa sổ vẫn là thao tác riêng từ nút × và menu.
- Đóng gói 0.1.6 với các thay đổi đang có trên branch, gồm ghi phím tắt, Dock tự ẩn, preview, Quick Panel và Tổng quan.

## Phát hành

**Beta ready.** Bản phát hành đánh dấu pre-release để phản ánh phần kiểm thử còn thiếu, nhất là DPI hỗn hợp, sleep/wake và frame timing trên các hiệu ứng khác nhau.
