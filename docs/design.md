# Tham chiếu thiết kế

Azr dùng tài liệu chính thức của Apple làm tham chiếu cho bố cục và tỉ lệ; phần dựng hình và tương tác được triển khai riêng trên Windows.

- [App icons](https://developer.apple.com/design/human-interface-guidelines/app-icons/): biểu tượng ứng dụng có nền vuông và được áp dụng mặt nạ bo góc nhất quán. Azr dùng đường cong superellipse liên tục; đây là phép xấp xỉ riêng, không phải mặt nạ chính thức của macOS.
- [Layout](https://developer.apple.com/design/human-interface-guidelines/layout): nội dung, lề và khoảng cách được phân bổ theo không gian khả dụng.
- [Typography](https://developer.apple.com/design/human-interface-guidelines/typography): bố cục cần thích ứng với kích thước chữ và ngôn ngữ. Azr đo chữ bằng bộ dựng thực tế, ngắt dòng nhãn và chú thích; hàng có điều khiển rộng được xếp dọc.

Ảnh xem trước có cột chú thích riêng. Tên tài khoản, ô nhập và giá trị dài được giới hạn trong vùng của chúng. Tỉ lệ hiển thị được giới hạn theo kích thước màn hình để cửa sổ Cài đặt không bị đẩy ra ngoài vùng nhìn thấy.

Azr dùng Inter và cơ chế phông chữ của Windows. Không phân phối phông chữ hệ thống của Apple. Nền kính trong phiên từ xa có thể dùng alpha, viền và phản sáng thay cho làm mờ backdrop đầy đủ. Các màu đã nằm trong ảnh biểu tượng gốc được giữ nguyên.
