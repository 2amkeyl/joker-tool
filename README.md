<div align="center">

<img src="banner.gif" alt="Joker" width="35%" />

# Joker Tool

Ứng dụng desktop (Windows) viết bằng Rust + egui, giao diện tiếng Việt.

[![Release](https://img.shields.io/github/v/release/2amkeyl/joker-tool?style=flat-square&color=e82036&v=2)](https://github.com/2amkeyl/Joker-Tool/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/2amkeyl/joker-tool/total?style=flat-square&color=e82036&v=2)](https://github.com/2amkeyl/Joker-Tool/releases)

</div>

## Tính năng

| Tab | Mô tả |
| --- | --- |
| **Cày Quest** | Tự động nhận và hoàn thành quest Discord |
| **Buff Badge** | Giả lập giờ chơi + số game để mở khoá badge |
| **HypeSquad** | Đổi / gỡ nhà HypeSquad |

---

## Tải về

1. Mở trang **[Releases](https://github.com/2amkeyl/joker-tool/releases/latest)** (cột bên phải của repo, mục **Releases**).
2. Kéo xuống phần **Assets**, bấm vào **`JokerTool-windows-x64.zip`** để tải.
3. Giải nén file zip (chuột phải → *Extract All…*).
4. Mở **`JokerTool.exe`**.

Link tải trực tiếp bản mới nhất:

```
https://github.com/2amkeyl/joker-tool/releases/latest/download/JokerTool-windows-x64.zip
```

## Yêu cầu

- Windows 10 hoặc 11 (64-bit)
- Một tài khoản Discord và User Token của tài khoản đó

## Lần đầu mở app

Windows có thể hiện cảnh báo xanh **"Windows protected your PC"**.
Bấm **More info** → **Run anyway**.

## Sử dụng

1. Dán token vào ô **Discord Token** → bấm **Kiểm tra**. Nếu hợp lệ, app hiện tên và avatar của bạn.
2. Bấm **Lưu** nếu muốn nhớ token cho lần sau.
3. Chọn tab (Cày Quest / Buff Badge / HypeSquad) rồi bấm nút bắt đầu.
4. Theo dõi tiến độ ở khung giữa và **Nhật ký hoạt động** bên dưới.
5. Đóng app = dừng mọi tác vụ.

## Dữ liệu lưu ở đâu?

| Mục | Đường dẫn |
| --- | --- |
| Token đã lưu | `%APPDATA%\JokerTool\config.json` |
| Nhật ký | `%APPDATA%\JokerTool\log.txt` |

Muốn xoá token: mở thư mục trên rồi xoá `config.json`. Token chỉ được gửi tới `discord.com`; avatar được tải từ `cdn.discordapp.com`.

## Lỗi thường gặp

| Lỗi | Cách xử lý |
| --- | --- |
| Token không hợp lệ | Token sai hoặc đã hết hạn, lấy lại token mới |
| Bị Cloudflare chặn (403) | Thử lại sau vài phút hoặc đổi mạng |
| Rate limit (429) | Đợi 5–10 phút rồi thử lại |
| Đứng ở "Đang xác thực" | Kiểm tra mạng, thử tắt VPN |

---

## Cảnh báo

- Việc dùng User Token để tự động hoá tài khoản **vi phạm Điều khoản dịch vụ của Discord**. Tài khoản có thể bị khoá hoặc ban.
- Nếu vẫn dùng, hãy dùng **tài khoản phụ**, không dùng tài khoản chính.
- Không chia sẻ token hay file `config.json` cho bất kỳ ai.
- Bạn tự chịu hoàn toàn trách nhiệm về rủi ro khi sử dụng. Dự án này không liên kết với Discord.

## Giấy phép

Mã nguồn được phát hành theo giấy phép **MIT**, xem file [LICENSE](LICENSE).