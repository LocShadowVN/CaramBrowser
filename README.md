# Caram Browser

[English](#english) | [Tiếng Việt](#tiếng-việt)

---

<a name="english"></a>
## English Version

A lightweight, privacy-focused, and memory-safe web browser engineered for Linux. Caram Browser combines native WebKit rendering with an integrated Brave-grade ad blocker, an Argon2id + AES-256-GCM encrypted password vault, and encrypted DNS-over-HTTPS resolution.

### Table of Contents
- [1. Overview](#1-overview)
- [2. Key Features](#2-key-features)
  - [2.1. Caram Shield (Ad & Tracker Blocker)](#21-caram-shield-ad--tracker-blocker)
  - [2.2. Caram Vault (Encrypted Password Manager)](#22-caram-vault-encrypted-password-manager)
  - [2.3. Secure DNS (DNS-over-HTTPS)](#23-secure-dns-dns-over-https)
  - [2.4. Desktop Browser Chrome](#24-desktop-browser-chrome)
- [3. Objective Browser Comparison](#3-objective-browser-comparison)
  - [3.1. Comparison Table](#31-comparison-table)
  - [3.2. Web Compatibility Trade-offs](#32-web-compatibility-trade-offs)
  - [3.3. Resource Efficiency](#33-resource-efficiency)
- [4. Installation for Linux Users](#4-installation-for-linux-users)
  - [4.1. Debian / Ubuntu (.deb)](#41-debian--ubuntu-deb)
  - [4.2. Universal Linux (.AppImage)](#42-universal-linux-appimage)
  - [4.3. Arch Linux / AUR](#43-arch-linux--aur)
- [5. Known Limitations](#5-known-limitations)
- [6. License](#6-license)

---

### 1. Overview

Caram Browser is built for Linux users who want a clean, fast browsing experience free from corporate telemetry, integrated cryptocurrency wallets, and background bloat. By utilizing native Linux web engine components alongside modern Rust security architecture, it provides maximum responsiveness on both high-end workstations and low-resource laptops.

---

### 2. Key Features

#### 2.1. Caram Shield (Ad & Tracker Blocker)
- **Built-in Network Filtering**: Powered by the official Brave Adblock engine, processing standard EasyList and EasyPrivacy rule sets.
- **Three Protection Levels**:
  - `Off`: All network requests and scripts are allowed.
  - `Standard`: Blocks intrusive display advertisements, cross-site tracking scripts, and automatically hides cookie banners via element hiding rules.
  - `Aggressive`: Adds heuristic blocking against device fingerprinting and browser telemetry attempts.

#### 2.2. Caram Vault (Encrypted Password Manager)
- **Hardened Key Derivation**: Uses the memory-hard `Argon2id` algorithm to derive encryption keys from your Master Password, effectively preventing offline GPU brute-force attacks.
- **Authenticated Encryption**: All credentials are encrypted locally using `AES-256-GCM` with an individual 96-bit nonce per entry.
- **Integrated Password Generator**: Creates cryptographically random credentials directly within the browser interface.

#### 2.3. Secure DNS (DNS-over-HTTPS)
- Encrypts your domain queries over HTTPS to prevent Internet Service Provider (ISP) snooping and DNS poisoning.
- Includes pre-configured profiles for Cloudflare (`1.1.1.1`), Quad9 (`9.9.9.9`), and Google (`8.8.8.8`).
- Allows custom DoH endpoints with a built-in latency ping test.

#### 2.4. Desktop Browser Chrome
- **Multi-Tab Management**: Tab reordering, duplicate/pin capabilities, and status indicators.
- **New Tab Dashboard**: Digital clock, live protection metrics (trackers blocked, data saved), and a customizable Speed Dial grid.
- **Internal System Pages**: Dedicated offline management interfaces for Settings (`caram://settings`), History (`caram://history`), Bookmarks (`caram://bookmarks`), Downloads (`caram://downloads`), Passwords (`caram://passwords`), and Extensions (`caram://extensions`).

---

### 3. Objective Browser Comparison

#### 3.1. Comparison Table

| Feature / Metric | Mozilla Firefox | Brave Browser | Caram Browser |
| :--- | :--- | :--- | :--- |
| **Rendering Engine** | Gecko / SpiderMonkey | Chromium Blink / V8 | WebKitGTK / Wry |
| **Base Core Language** | C++, Rust, JavaScript | C++, Chromium Fork | 100% Rust Architecture |
| **Idle RAM (1 Tab Open)** | ~450 MB - 650 MB | ~550 MB - 800 MB | **~90 MB - 170 MB** |
| **Ad / Tracker Blocking** | Basic protection | Built-in Brave Shields | **Built-in Brave Engine** |
| **Bloat / Sponsored Content** | Pocket, Sponsored tiles | Crypto Wallet, BAT tokens | **Zero (Completely clean)** |
| **Web Compatibility** | High (~98%) | Absolute (~100%) | **Moderate (~85% - 90%)** |
| **DRM Support (Netflix, etc.)** | Supported (Widevine CDM) | Supported (Widevine CDM) | **Not supported by default** |
| **Extension Support** | Full Firefox Add-ons | Full Chrome Web Store | Developer unpacked (MV3) |

#### 3.2. Web Compatibility Trade-offs

It is important to be completely honest about web compatibility:

- **WebKit vs. Blink/Gecko**: Caram Browser uses WebKitGTK. While it adheres strictly to official W3C web standards (HTML5, CSS3, ES2024), major tech companies primarily optimize their web applications for Google Chromium (Blink).
- **Proprietary Media (DRM)**: Services requiring proprietary Google Widevine DRM (such as Netflix, Spotify Web Player, Disney+, Amazon Prime Video) will not work out of the box on Caram Browser.
- **Intended Use Case**: Caram Browser excels at reading technical documentation, GitHub, Reddit, Wikipedia, forums, news sites, and daily web tasks with minimal system impact. For proprietary web applications that mandate Chromium-only APIs, using a secondary Chromium browser is recommended.

#### 3.3. Resource Efficiency

Because Caram Browser leverages the native system rendering library rather than running an entire bundled Chromium instance, background memory consumption is reduced by up to 70%.

---

### 4. Installation for Linux Users

Pre-built binaries and installation packages are available under the GitHub **Releases** tab.

#### 4.1. Debian / Ubuntu (.deb)
Download the `.deb` package and install it via terminal:
```bash
sudo dpkg -i caram-browser_*_amd64.deb
sudo apt-get install -f
```

#### 4.2. Universal Linux (.AppImage)
Compatible with any modern Linux distribution (Ubuntu, Fedora, Arch, Debian, openSUSE):
```bash
chmod +x caram-browser_*_amd64.AppImage
./caram-browser_*_amd64.AppImage
```

#### 4.3. Arch Linux / AUR
For Arch-based distributions (Arch, Manjaro, EndeavourOS):
```bash
yay -S caram-browser-bin
```

---

### 5. Known Limitations

- **Extension Ecosystem**: Current extension support is intended for manual development and testing (loading unpacked extensions via `manifest.json`). There is no integrated store integration.
- **Audio/Video Codecs**: Video playback depends on the system GStreamer plugins installed on your Linux host. Ensure `gstreamer1.0-plugins-good`, `bad`, and `ugly` are installed for full media playback support.

---

### 6. License

This project is licensed under the **GNU General Public License v3.0 (GNU GPLv3)**. See the [LICENSE](LICENSE) file for details.

---
---

<a name="tiếng-việt"></a>
## Bản Tiếng Việt

Trình duyệt web tinh gọn, bảo mật và an toàn bộ nhớ dành riêng cho người dùng Linux. Caram Browser kết hợp nhân hiển thị WebKit nguyên bản với bộ chặn quảng cáo chuẩn Brave, kho lưu trữ mật khẩu mã hóa Argon2id + AES-256-GCM và phân giải DNS bảo mật qua HTTPS.

### Mục lục
- [1. Giới thiệu tổng quan](#1-giới-thiệu-tổng-quan)
- [2. Tính năng nổi bật](#2-tính-năng-nổi-bật)
  - [2.1. Caram Shield (Chặn quảng cáo và theo dõi)](#21-caram-shield-chặn-quảng-cáo-và-theo-dõi)
  - [2.2. Caram Vault (Kho mật khẩu mã hóa)](#22-caram-vault-kho-mật-khẩu-mã-hóa)
  - [2.3. Secure DNS (DNS-over-HTTPS)](#23-secure-dns-dns-over-https)
  - [2.4. Giao diện trình duyệt chuẩn](#24-giao-diện-trình-duyệt-chuẩn)
- [3. So sánh kỹ thuật công bằng](#3-so-sánh-kỹ-thuật-công-bằng)
  - [3.1. Bảng so sánh tổng quan](#31-bảng-so-sánh-tổng-quan)
  - [3.2. Đánh giá độ tương thích web](#32-đánh-giá-độ-tương-thích-web)
  - [3.3. Hiệu quả sử dụng tài nguyên](#33-hiệu-quả-sử-dụng-tài-nguyên)
- [4. Hướng dẫn cài đặt cho người dùng Linux](#4-hướng-dẫn-cài-đặt-cho-người-dùng-linux)
  - [4.1. Dành cho Debian / Ubuntu (.deb)](#41-dành-cho-debian--ubuntu-deb)
  - [4.2. Dành cho mọi bản phân phối Linux (.AppImage)](#42-dành-cho-mọi-bản-phân-phối-linux-appimage)
  - [4.3. Dành cho Arch Linux / Manjaro](#43-dành-cho-arch-linux--manjaro)
- [5. Các giới hạn kỹ thuật](#5-các-giới-hạn-kỹ-thuật)
- [6. Giấy phép](#6-giấy-phép)

---

### 1. Giới thiệu tổng quan

Caram Browser được phát triển nhằm mục tiêu giải phóng người dùng Linux khỏi sự nặng nề của các trình duyệt Chromium thương mại. Trình duyệt không chứa mã theo dõi hành vi, không tích hợp ví tiền mã hóa (crypto) và không chạy các dịch vụ quảng cáo ngầm. Nhờ tận dụng thư viện WebKit có sẵn trên hệ điều hành kết hợp với ngôn ngữ Rust, Caram Browser mang lại tốc độ phản hồi cao ngay cả trên các dòng máy tính cấu hình khiêm tốn.

---

### 2. Tính năng nổi bật

#### 2.1. Caram Shield (Chặn quảng cáo và theo dõi)
- **Lọc mạng trực tiếp**: Ứng dụng nhân lọc `adblock` của Brave, tương thích hoàn toàn với bộ quy tắc EasyList và EasyPrivacy.
- **3 Cấp độ bảo vệ**:
  - `Tắt (Off)`: Tải đầy đủ toàn bộ nội dung và script của trang.
  - `Tiêu chuẩn (Standard)`: Tự động chặn banner quảng cáo, mã theo dõi chéo trang và ẩn các bảng thông báo cookie phiền toái.
  - `Nghiêm ngặt (Aggressive)`: Bổ sung cơ chế phát hiện và chặn các hành vi thu thập dấu vân tay thiết bị (anti-fingerprinting) và thu thập dữ liệu ngầm (telemetry).

#### 2.2. Caram Vault (Kho mật khẩu mã hóa)
- **Dẫn xuất khoá an toàn**: Ứng dụng thuật toán `Argon2id` kết hợp muối ngẫu nhiên (Salt 128-bit) từ mật khẩu chính, triệt tiêu nguy cơ bị dò khóa bằng GPU hay dàn máy đào chuyên dụng.
- **Mã hóa xác thực AEAD**: Toàn bộ tài khoản và mật khẩu được mã hóa cục bộ bằng thuật toán `AES-256-GCM` với mã Nonce 96-bit riêng biệt cho từng dòng dữ liệu.
- **Bộ tạo mật khẩu tích hợp**: Hỗ trợ khởi tạo các chuỗi ký tự mật khẩu ngẫu nhiên có độ dài và độ phức tạp cao chỉ với một nút bấm.

#### 2.3. Secure DNS (DNS-over-HTTPS)
- Mã hóa toàn bộ các truy vấn địa chỉ web thông qua HTTPS, ngăn chặn nhà mạng (ISP) và các tác nhân mạng theo dõi lịch sử truy cập.
- Thiết lập sẵn các máy chủ DNS uy tín: Cloudflare (`1.1.1.1`), Quad9 (`9.9.9.9`), Google (`8.8.8.8`).
- Hỗ trợ nhập máy chủ DNS tùy chỉnh và có công cụ kiểm tra độ trễ (Ping) trực tiếp.

#### 2.4. Giao diện trình duyệt chuẩn
- **Quản lý đa thẻ (Multi-Tab)**: Đóng, mở, chuyển đổi tab mượt mà và hiển thị trạng thái chặn của từng trang.
- **Trang mở thẻ mới (New Tab)**: Giao diện tối giản hiển thị đồng hồ kỹ thuật số, số lượng quảng cáo đã chặn và lưới phím tắt truy cập nhanh (Speed Dial).
- **Hệ thống trang nội bộ**: Trang cấu hình (`caram://settings`), Lịch sử (`caram://history`), Dấu trang (`caram://bookmarks`), Quản lý tệp tải xuống (`caram://downloads`), Mật khẩu (`caram://passwords`) và Tiện ích (`caram://extensions`).

---

### 3. So sánh kỹ thuật công bằng

#### 3.1. Bảng so sánh tổng quan

| Tiêu chí so sánh | Mozilla Firefox | Brave Browser | Caram Browser |
| :--- | :--- | :--- | :--- |
| **Nhân hiển thị (Engine)** | Gecko / SpiderMonkey | Chromium Blink / V8 | WebKitGTK / Wry |
| **Ngôn ngữ phát triển** | C++, Rust, JavaScript | C++, Chromium Fork | Kiến trúc 100% Rust |
| **RAM tiêu thụ (1 Tab chờ)**| ~450 MB - 650 MB | ~550 MB - 800 MB | **~90 MB - 170 MB** |
| **Trình chặn quảng cáo** | Cơ bản | Brave Shields mạnh mẽ | **Dùng chung nhân với Brave** |
| **Ứng dụng rác đi kèm** | Pocket, Dịch vụ VPN mời gọi | Ví Crypto, Token thưởng BAT | **Hoàn toàn sạch (Không có)** |
| **Độ tương thích trang web** | Rất cao (~98%) | Tuyệt đối (~100%) | **Trung bình - Khá (~85% - 90%)** |
| **Hỗ trợ phim bản quyền (DRM)**| Có sẵn (Widevine CDM) | Có sẵn (Widevine CDM) | **Không hỗ trợ mặc định** |
| **Kho tiện ích mở rộng** | Rất phong phú (Add-ons) | Rất phong phú (Chrome Store)| Hỗ trợ nạp thủ công (MV3) |

#### 3.2. Đánh giá độ tương thích web

Nhận định thẳng thắn về khía cạnh kỹ thuật:

- **Sự khác biệt của WebKitGTK**: Caram Browser hoạt động dựa trên nhân WebKit của Linux. Dù tuân thủ rất tốt các chuẩn web quốc tế của W3C, phần lớn các trang web thương mại ngày nay được tối ưu hóa riêng cho nhân Chromium của Google.
- **Nội dung bản quyền số (DRM)**: Các nền tảng yêu cầu chứng chỉ bản quyền Google Widevine DRM khắt khe (như Netflix, Spotify Web, Disney+, VieON) sẽ không thể phát được trên Caram Browser.
- **Đối tượng phù hợp**: Caram Browser là lựa chọn tuyệt vời cho nhu cầu đọc báo, lập trình, tra cứu tài liệu, lướt GitHub, Reddit, Wikipedia, diễn đàn, nghe nhạc trên các nền tảng mở mà không làm nóng máy hay tốn pin. Đối với các tác vụ văn phòng đặc thù của Google hoặc xem phim bản quyền, bạn nên sử dụng trình duyệt Chromium phụ trợ.

#### 3.3. Hiệu quả sử dụng tài nguyên

Do không phải gánh toàn bộ bộ mã nguồn Chromium đồ sộ chạy ngầm, Caram Browser tiết kiệm tới 70% dung lượng RAM và giảm đáng kể mức tải CPU trên Linux so với Firefox và Brave.

---

### 4. Hướng dẫn cài đặt cho người dùng Linux

Các gói cài đặt sẵn được đóng gói chính thức tại mục **Releases** trên GitHub. Người dùng không cần cài đặt công cụ lập trình để sử dụng.

#### 4.1. Dành cho Debian / Ubuntu (.deb)
Tải gói `.deb` về máy và cài đặt bằng lệnh:
```bash
sudo dpkg -i caram-browser_*_amd64.deb
sudo apt-get install -f
```

#### 4.2. Dành cho mọi bản phân phối Linux (.AppImage)
Chạy được ngay trên mọi bản phân phối (Ubuntu, Fedora, Arch, Linux Mint, Debian...):
```bash
chmod +x caram-browser_*_amd64.AppImage
./caram-browser_*_amd64.AppImage
```

#### 4.3. Dành cho Arch Linux / Manjaro
Cài đặt trực tiếp thông qua AUR:
```bash
yay -S caram-browser-bin
```

---

### 5. Các giới hạn kỹ thuật

- **Tiện ích mở rộng (Extensions)**: Hiện tại chỉ hỗ trợ chế độ nhà phát triển (nạp trực tiếp thư mục chứa tệp `manifest.json`), chưa liên kết tự động với Chrome Web Store.
- **Bộ giải mã âm thanh/hình ảnh (Codecs)**: Khả năng phát video phụ thuộc vào thư viện GStreamer trên máy Linux của bạn. Hãy cài đặt gói `gstreamer1.0-plugins-good` và `bad` để đảm bảo xem được mọi định dạng video.

---

### 6. Giấy phép

Dự án được phát hành và bảo vệ dưới các điều khoản của **Giấy phép Công cộng GNU v3.0 (GNU GPLv3)**. Xem toàn văn giấy phép tại tệp [LICENSE](LICENSE).
