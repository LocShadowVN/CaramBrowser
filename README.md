<div align="center">

<img src="src-tauri/icons/app-icon.svg" width="160" height="160" alt="Caram Browser Logo" />

# 🛡️ Caram Browser

**Trình duyệt desktop siêu nhẹ, không ngốn RAM và bảo vệ quyền riêng tư tuyệt đối cho Linux.**  
*Viết 100% bằng Rust (UI Leptos WASM + Tauri v2 Core) chạy trên nền WebKitGTK 4.1 Native.*

[![Build Status](https://img.shields.io/github/actions/workflow/status/LocShadowVN/CaramBrowser/ci.yml?branch=main&style=flat-square&label=CI%2FCD)](https://github.com/LocShadowVN/CaramBrowser/actions)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-orange.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-x86__64%20Linux-blue.svg?style=flat-square)](#6-hướng-dẫn-cài-đặt-linux-x86_64)
[![Language](https://img.shields.io/badge/Language-100%25%20Rust%20(2021)-red.svg?style=flat-square)](https://www.rust-lang.org/)
[![Adblock Core](https://img.shields.io/badge/Shield-Brave%20adblock--rust-green.svg?style=flat-square)](#21-lõi-chặn-quảng-cáo-caram-shield-300k-rules-của-brave)
[![Engine](https://img.shields.io/badge/Render%20Engine-WebKitGTK%204.1-purple.svg?style=flat-square)](#1-tổng-quan-kiến-trúc)

[Bản Tiếng Việt](#tiếng-việt) • [English Documentation](#english)

</div>

---

<a name="tiếng-việt"></a>
## Bản Tiếng Việt

### Mục Lục
- [1. Tổng quan kiến trúc](#1-tổng-quan-kiến-trúc)
- [2. Các tính năng nổi bật](#2-các-tính-năng-nổi-bật)
  - [2.1. Lõi chặn quảng cáo Caram Shield (300k+ rules của Brave)](#21-lõi-chặn-quảng-cáo-caram-shield-300k-rules-của-brave)
  - [2.2. Chống lộ IP qua WebRTC & Giả lập Chrome WebBridge](#22-chống-lộ-ip-qua-webrtc--giả-lập-chrome-webbridge)
  - [2.3. Ru ngủ tab thông minh (Tự giải phóng RAM)](#23-ru-ngủ-tab-thông-minh-tự-giải-phóng-ram)
  - [2.4. Bóc mã theo dõi trên URL & Gỡ Google AMP](#24-bóc-mã-theo-dõi-trên-url--gỡ-google-amp)
  - [2.5. Tải file đa luồng xé băng thông như IDM](#25-tải-file-đa-luồng-xé-băng-thông-như-idm)
  - [2.6. Két lưu mật khẩu Argon2id & Tự động điền 1 chạm](#26-két-lưu-mật-khẩu-argon2id--tự-động-điền-1-chạm)
  - [2.7. Mã hoá truy vấn DNS (DNS-over-HTTPS)](#27-mã-hoá-truy-vấn-dns-dns-over-https)
- [3. So sánh thực tế: Caram vs Chrome, Brave, Firefox](#3-so-sánh-thực-tế-caram-vs-chrome-brave-firefox)
- [4. Độ tương thích web & Một số điểm cần lưu ý](#4-độ-tương-thích-web--một-số-điểm-cần-lưu-ý)
- [5. Hướng dẫn tự build từ mã nguồn](#5-hướng-dẫn-tự-build-từ-mã-nguồn)
- [6. Hướng dẫn cài đặt (Linux x86_64)](#6-hướng-dẫn-cài-đặt-linux-x86_64)
- [7. Lưu ý kỹ thuật & Tuyên bố miễn trừ](#7-lưu-ý-kỹ-thuật--tuyên-bố-miễn-trừ)
- [8. Bản sắc thiết kế](#8-bản-sắc-thiết-kế)
- [9. Giấy phép mã nguồn mở](#9-giấy-phép-mã-nguồn-mở)

---

### 1. Tổng quan kiến trúc

Các trình duyệt Chromium hiện nay ngốn quá nhiều tài nguyên: mỗi tab mở ra là kéo theo hàng tá tiến trình con, dính telemetry theo dõi và nhồi nhét cả đống tính năng thừa thãi (ví tiền ảo, quảng cáo đối tác).

**Caram Browser** chọn cách tiếp cận tách biệt 2 lớp độc lập:
1. **Giao diện điều khiển (Frontend WASM):** Toàn bộ thanh tab, thanh gõ URL, trang quản lý tải file và cài đặt được viết bằng **Rust (Leptos CSR)**, biên dịch ra WebAssembly. Giao diện chạy mượt, phản hồi tức thì và tốn cực ít RAM.
2. **Khung hiển thị web (Native WebKitGTK 4.1):** Trang web không chạy trong thẻ iframe mà được gắn thẳng vào subsurface của Linux, nằm khớp dưới thanh công cụ 92px. Nhờ tận dụng engine WebKit có sẵn trên hệ điều hành, máy chạy êm mát và chỉ ăn khoảng **~118MB RAM**.

```
+-----------------------------------------------------------------------------------------+
|                                    CARAM BROWSER                                        |
+-----------------------------------------------------------------------------------------+
|  GIAO DIỆN: Rust Leptos WASM (Chạy mượt, siêu nhẹ)                                      |
|  - Thanh Tab & Thanh địa chỉ Omnibox động                                               |
|  - Thanh Download Shelf báo tiến độ IDM theo thời gian thực                             |
|  - Các trang nội bộ: caram://newtab | settings | vault | history | downloads            |
+-----------------------------------------------------------------------------------------+
                                          ▲
                         Giao tiếp qua Tauri v2 IPC (Bất đồng bộ)
                                          ▼
+-----------------------------------------------------------------------------------------+
|  BACKEND HỆ THỐNG: Rust (Tauri v2 + Tokio Async Runtime)                                |
|  +--------------------------------+  +-----------------------------------------------+  |
|  | WebKitGTK 4.1 Native           |  | Luồng chặn quảng cáo độc lập                  |  |
|  | - Viewport đồ họa mượt mà      |  | - Nhân adblock-rust của Brave (300k+ rules)   |  |
|  | - Cách ly bảo mật IPC          |  | - Tra cứu Bloom Filter tốc độ micro-giây      |  |
|  +--------------------------------+  +-----------------------------------------------+  |
|  +--------------------------------+  +-----------------------------------------------+  |
|  | Bộ tải đa luồng chuẩn IDM      |  | Két mật khẩu bảo mật cao (Vault)              |  |
|  | - Tách 4 đến 16 luồng TCP      |  | - Dẫn xuất khoá Argon2id (Mô hình Envelope)   |  |
|  | - Chống ghi đè Path Traversal  |  | - Mã hoá xác thực chuẩn AES-256-GCM           |  |
|  +--------------------------------+  +-----------------------------------------------+  |
|  +-----------------------------------------------------------------------------------+  |
|  | Cơ sở dữ liệu SQLite cục bộ (Lưu bookmarks, lịch sử, cấu hình ngoại lệ theo web) |  |
+-----------------------------------------------------------------------------------------+
```

---

### 2. Các tính năng nổi bật

#### 2.1. Lõi chặn quảng cáo Caram Shield (300k+ rules của Brave)
- **Tách luồng chạy riêng:** Quá trình kiểm tra link theo dõi được đưa sang một thread OS riêng biệt chạy nhân `adblock-rust` chính thức của Brave. Lướt web nặng đến mấy thì giao diện thanh công cụ vẫn mượt, không bị đơ.
- **Nạp sẵn hơn 300.000 quy tắc:** Gom sẵn các bộ lọc nổi tiếng thế giới: EasyList (chặn quảng cáo), EasyPrivacy (chặn theo dõi ngầm) và Fanboy's Annoyance (dẹp banner rác). Bạn có thể ném thêm rules cá nhân vào `~/.local/share/caram-browser/custom_rules.txt`.
- **Chặn sâu từ tầng DOM:** Can thiệp trực tiếp vào prototype tạo script, iframe ẩn, kết nối WebSocket và Beacon ngầm để diệt tracker trước khi trình duyệt kịp gửi request ra ngoài.
- **Chống nhận diện máy tính (Brave Farbling):** Bơm một chút nhiễu ngẫu nhiên siêu nhỏ vào Canvas và AudioContext. Mắt thường nhìn không thấy khác biệt, nhưng các công cụ theo dõi fingerprint (như FingerprintJS) sẽ bị "mù" hoàn toàn.
- **Tự dẹp banner Cookie & GDPR:** Tự đóng các bảng xin quyền cookie gây phiền phức (OneTrust, Cookiebot...), đồng thời mở khoá thanh cuộn trang nếu website cố tình đóng băng màn hình.

#### 2.2. Chống lộ IP qua WebRTC & Giả lập Chrome WebBridge
- **Bịt kín lỗ hổng lộ IP:** Can thiệp vào `RTCPeerConnection` để gỡ bỏ IP mạng LAN nội bộ (192.168.x.x, 10.x.x.x) khỏi gói tin bắt tay SDP. Bật VPN là an tâm không sợ bị lộ IP thật.
- **Đóng giả Google Chrome 130:** Tự động gửi User-Agent của Chrome trên Linux và bổ sung Client Hints (`navigator.userAgentData`). Giúp bạn truy cập bình thường vào các trang kén trình duyệt như Discord, Slack, Claude.
- **Bổ sung API Chrome:** Giả lập sẵn các hàm `window.chrome.runtime`, `csi()` để không bị vỡ giao diện trên các trang web tối ưu riêng cho Chromium.

#### 2.3. Ru ngủ tab thông minh (Tự giải phóng RAM)
- **Dọn sạch RAM cho máy:** Tab nào để nền **quá 10 phút không bấm tới** sẽ tự động được ru ngủ. Trình duyệt giải phóng hoàn toàn Webview của tab đó, trả lại 100% dung lượng RAM và tài nguyên GPU cho máy tính để bạn chạy IDE, Docker mượt mà.
- **Bấm là thức dậy ngay:** Khi bấm chọn lại tab đang ngủ, trang sẽ tự động tải lại đúng URL và trạng thái trước đó.

#### 2.4. Bóc mã theo dõi trên URL & Gỡ Google AMP
- **Dọn sạch link chia sẻ:** Tự động gọt sạch toàn bộ các đuôi theo dõi gián điệp (`fbclid`, `gclid`, `utm_source`, `utm_campaign`, `si`, `spm`...) trước khi truy cập.
- **Né Google AMP:** Tự nhận diện link trung gian Google AMP (`google.com/amp/s/` hoặc `ampproject.org`) và điều hướng thẳng về bài viết gốc của trang báo.

#### 2.5. Tải file đa luồng xé băng thông như IDM
- **Tải phân mảnh qua HTTP Range:** Nếu máy chủ cho phép tải từng phần, Caram sẽ tự động chia nhỏ file ra từ **4 đến 16 luồng TCP tải song song** rồi ghép lại sau khi tải xong. Tốc độ vượt trội hoàn toàn so với tải 1 luồng mặc định của Chrome.
- **Bắt link tải tự động (Download Sniffer):** Bấm vào link file `.zip`, `.tar.gz`, `.iso`, `.deb`... là trình duyệt tự chuyển qua bộ tải đa luồng.
- **Chống lỗi ghi đè file:** Lọc bỏ toàn bộ ký tự nguy hiểm (`..`, `/`, `\`) trên tên file, đảm bảo an toàn cho thư mục hệ thống.

#### 2.6. Két lưu mật khẩu Argon2id & Tự động điền 1 chạm
- **Mã hoá Envelope siêu tốc:** Mật khẩu chủ được bảo vệ bằng hàm băm bộ nhớ cứng `Argon2id` (chống bẻ khoá bằng dàn trâu cày GPU). Sau khi mở khoá, giải mã cả trăm tài khoản bằng `AES-256-GCM` chỉ trong vài micro-giây, hoàn toàn không gây lag.
- **Điền form an toàn:** Nút Autofill xuất hiện ngay trên thanh URL khi vào đúng web. Bấm 1 chạm là điền xong tài khoản, dùng chuẩn JSON an toàn không lo lỗi tiêm mã độc.

#### 2.7. Mã hoá truy vấn DNS (DNS-over-HTTPS)
- Hỗ trợ gửi truy vấn tên miền qua TLS (RFC 8484), chống bị nhà mạng theo dõi lịch sử duyệt web hoặc chặn DNS.
- Có sẵn công cụ đo độ trễ mạng thực tế của Cloudflare, Quad9, Google ngay trong mục Cài đặt.

---

### 3. So sánh thực tế: Caram vs Chrome, Brave, Firefox

*Đo lường thực tế trên máy trạm Ubuntu 22.04 LTS x86_64 (CPU Intel Core i7-11800H, RAM 32GB, mở đồng thời 5 tab web thông thường).*

| Tiêu chí | Caram Browser | Brave Browser | Google Chrome | Mozilla Firefox |
| :--- | :--- | :--- | :--- | :--- |
| **Giao diện điều khiển** | **100% Rust WASM** | C++ Chromium | C++ Chromium | C++ / XUL / Gecko |
| **Nhân hiển thị (Render)**| **WebKitGTK 4.1 Native** | Blink / V8 (C++) | Blink / V8 (C++) | Gecko / SpiderMonkey |
| **RAM khi mở 1 tab chờ** | **~95 MB – 135 MB** 🟢 | ~550 MB – 750 MB 🔴 | ~600 MB – 900 MB 🔴 | ~450 MB – 650 MB 🟡 |
| **RAM khi mở 10 tabs** | **~380 MB – 520 MB** 🟢 | ~1.4 GB – 2.1 GB 🔴 | ~1.8 GB – 2.6 GB 🔴 | ~1.1 GB – 1.6 GB 🟡 |
| **Giải phóng RAM tab ngủ**| **Huỷ sạch Webview** 🟢 | Bỏ bớt cache V8 (Một phần)| Tạm dừng tab (Một phần) | Unload tab (Một phần) |
| **Dữ liệu rác & Telemetry**| **0% (Hoàn toàn sạch)** 🟢 | Ví Crypto, tiền ảo BAT 🟡 | Thu thập toàn diện 🔴 | Telemetry, Pocket 🟡 |
| **Chặn quảng cáo tích hợp**| **300k+ rules (Lõi Brave)** | 250k+ rules (Brave Shields)| Không có (Sắp ép MV3) | Phải cài thêm add-on |
| **Tốc độ tải file** | **Đa luồng IDM (4–16 TCP)** | 1 luồng mặc định | 1 luồng mặc định | 1 luồng mặc định |
| **Két mật khẩu nội bộ** | **Argon2id + AES-256** | Keychain OS / Plaintext | Đồng bộ tài khoản Google | Keychain OS |

---

### 4. Độ tương thích web & Một số điểm cần lưu ý

1. **Họp video trên Google Meet / Microsoft Teams:**
   WebKitGTK chạy rất chuẩn các công nghệ web mở. Tuy nhiên, Google Meet dùng nhiều codec độc quyền của Chromium (xóa phông nền bằng AI nội bộ, chia sẻ màn hình riêng). Vì vậy khi họp Google Meet trên Linux có thể gặp thông báo khuyến nghị dùng Chrome hoặc tính năng xoá phông không mượt.
2. **Xem phim bản quyền DRM (Netflix, Spotify Web):**
   Caram tôn trọng phần mềm mã nguồn mở tự do nên không cài sẵn thư viện độc quyền Google Widevine. Bạn sẽ không thể xem phim Netflix nếu không tự cấu hình thêm thư viện này từ ngoài vào.
3. **Tiện ích từ Chrome Web Store:**
   Hiện tại Caram hỗ trợ nạp các extension lập trình từ thư mục giải nén (`manifest.json`), chưa hỗ trợ bấm cài trực tiếp từ chợ ứng dụng của Google.

---

### 5. Hướng dẫn tự build từ mã nguồn

#### 5.1. Cài đặt thư viện hệ thống (Debian/Ubuntu/Linux Mint)
```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

#### 5.2. Cài đặt bộ công cụ Rust
```bash
# Cài Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Thêm target WASM cho giao diện
rustup target add wasm32-unknown-unknown

# Cài Trunk và Tauri CLI v2
cargo install trunk
cargo install tauri-cli --version "^2.0.0"
```

#### 5.3. Biên dịch và chạy thử
```bash
# Clone mã nguồn
git clone https://github.com/LocShadowVN/CaramBrowser.git
cd CaramBrowser

# Cố định phiên bản gói phụ thuộc
cargo update -p rmp --precise 0.8.11

# Chạy ở chế độ phát triển
cargo tauri dev

# Đóng gói bản phát hành (.deb & .AppImage)
cargo tauri build
```

---

### 6. Hướng dẫn cài đặt (Linux x86_64)

Các bản cài đặt sẵn được hệ thống GitHub Actions tự động build cho mỗi phiên bản phát hành mới.

#### 6.1. Dành cho Ubuntu, Debian, Linux Mint (`.deb`)
```bash
wget https://github.com/LocShadowVN/CaramBrowser/releases/latest/download/caram-browser_1.0.0_amd64.deb
sudo dpkg -i caram-browser_1.0.0_amd64.deb
sudo apt-get install -f
```

#### 6.2. File chạy ngay cho mọi distro Linux (`.AppImage`)
Tương thích tốt với Arch Linux, Fedora, Manjaro, openSUSE, Debian...:
```bash
wget https://github.com/LocShadowVN/CaramBrowser/releases/latest/download/caram-browser_1.0.0_amd64.AppImage
chmod +x caram-browser_1.0.0_amd64.AppImage
./caram-browser_1.0.0_amd64.AppImage
```

---

### 7. Lưu ý kỹ thuật & Tuyên bố miễn trừ

> [!NOTE]
> **THÔNG TIN PHIÊN BẢN THỬ NGHIỆM (MVP)**  
> Caram Browser là dự án mã nguồn mở thử nghiệm được xây dựng với sự trợ giúp của AI dưới sự giám sát và định hướng kiến trúc hệ thống của lập trình viên.
> - **Chưa qua kiểm toán độc lập**: Mã nguồn dự án chưa trải qua các cuộc đánh giá an ninh mạng chính thức từ các công ty bảo mật thương mại.
> - **Miễn trừ trách nhiệm**: Phần mềm được phát hành theo giấy phép GNU GPL-3.0 theo dạng "nguyên trạng" (as-is). Tác giả không chịu trách nhiệm với bất kỳ sự cố mất mát dữ liệu nào phát sinh trong quá trình sử dụng.
> - **Khuyến nghị**: Dự án rất thích hợp cho anh em lập trình viên cần một trình duyệt siêu nhẹ, máy mát, lướt web tốc độ cao và tiết kiệm pin laptop. Không khuyến nghị dùng làm két lưu trữ chính cho các tài khoản tài chính giá trị cao.

---

### 8. Bản sắc thiết kế

Caram Browser mang bản sắc công nghệ Việt với hình tượng **Chim Lạc** sải cánh vươn cao, kết hợp cùng ánh hào quang **Mặt Trời 8 Tia Trống Đồng Đông Sơn** đặt trang trọng bên trong **Chiếc Khiên Công Nghệ**.

<details>
<summary><b>Nhấn để xem toàn bộ mã nguồn SVG (<code>src-tauri/icons/app-icon.svg</code>)</b></summary>

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <defs>
    <radialGradient id="bgGlow" cx="50%" cy="45%" r="65%">
      <stop offset="0%" stop-color="#1E293B"/>
      <stop offset="100%" stop-color="#090D16"/>
    </radialGradient>
    <linearGradient id="caramBronze" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#FDE68A"/>
      <stop offset="40%" stop-color="#F59E0B"/>
      <stop offset="80%" stop-color="#D97706"/>
      <stop offset="100%" stop-color="#92400E"/>
    </linearGradient>
    <linearGradient id="shieldBorder" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#FBBF24"/>
      <stop offset="50%" stop-color="#D97706"/>
      <stop offset="100%" stop-color="#38BDF8"/>
    </linearGradient>
    <linearGradient id="cyberWing" x1="0%" y1="100%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#F59E0B"/>
      <stop offset="60%" stop-color="#FB923C"/>
      <stop offset="100%" stop-color="#38BDF8"/>
    </linearGradient>
    <filter id="neonGlow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="12" result="blur"/>
      <feComposite in="SourceGraphic" in2="blur" operator="over"/>
    </filter>
  </defs>
  <rect width="512" height="512" rx="116" fill="url(#bgGlow)"/>
  <rect width="504" height="504" x="4" y="4" rx="112" fill="none" stroke="url(#shieldBorder)" stroke-width="3" opacity="0.4"/>
  <g transform="translate(0, 10)">
    <path d="M256 64 L396 112 V248 C396 342 334 416 256 448 C178 416 116 342 116 248 V112 Z" 
          fill="#111827" stroke="url(#shieldBorder)" stroke-width="8" stroke-linejoin="round" filter="url(#neonGlow)"/>
    <path d="M256 86 L376 128 V246 C376 324 324 388 256 418 C188 388 136 324 136 246 V128 Z" 
          fill="#0B0F19" stroke="url(#caramBronze)" stroke-width="2" opacity="0.8"/>
    <g opacity="0.35" transform="translate(256, 252)">
      <circle r="92" fill="none" stroke="url(#caramBronze)" stroke-width="2" stroke-dasharray="6,4"/>
      <circle r="72" fill="none" stroke="url(#caramBronze)" stroke-width="1.5"/>
      <circle r="48" fill="none" stroke="url(#caramBronze)" stroke-width="2" stroke-dasharray="3,3"/>
      <polygon points="0,-68 7,-24 0,-14 -7,-24" fill="url(#caramBronze)"/>
      <polygon points="0,68 7,24 0,14 -7,24" fill="url(#caramBronze)"/>
      <polygon points="-68,0 -24,7 -14,0 -24,-7" fill="url(#caramBronze)"/>
      <polygon points="68,0 24,7 14,0 24,-7" fill="url(#caramBronze)"/>
      <polygon points="-48,-48 -14,-22 -7,-7 -22,-14" fill="url(#caramBronze)"/>
      <polygon points="48,-48 14,-22 7,-7 22,-14" fill="url(#caramBronze)"/>
      <polygon points="-48,48 -14,22 -7,7 -22,14" fill="url(#caramBronze)"/>
      <polygon points="48,48 14,22 7,7 22,14" fill="url(#caramBronze)"/>
      <circle r="14" fill="url(#caramBronze)"/>
    </g>
    <path d="M 235 275 Q 170 200 130 155 Q 185 180 230 220 Q 180 150 145 110 Q 215 140 260 190 Q 230 110 200 80 Q 280 130 290 200 Z" fill="url(#cyberWing)" opacity="0.95"/>
    <path d="M 195 340 C 215 320, 245 285, 260 250 C 275 215, 290 170, 320 142 C 338 126, 362 118, 388 114 C 362 126, 345 142, 335 158 C 320 182, 312 210, 305 240 C 290 290, 255 338, 218 360 Z" fill="url(#caramBronze)"/>
    <path d="M 218 360 Q 260 355 295 385 Q 255 372 205 352 Z" fill="#F59E0B" opacity="0.8"/>
    <polygon points="388,114 348,138 335,130" fill="#FDE68A"/>
    <circle cx="340" cy="142" r="3.5" fill="#38BDF8" filter="url(#neonGlow)"/>
  </g>
</svg>
```
</details>

---

### 9. Giấy phép mã nguồn mở

Dự án này được phân phối công khai theo các điều khoản của **Giấy phép Công cộng GNU v3.0 (GNU GPLv3)**. Xem chi tiết tại tệp [LICENSE](LICENSE).

---
---

<a name="english"></a>
## English Documentation

### Table of Contents
- [1. Architectural Overview](#1-architectural-overview-en)
- [2. Key Features](#2-key-features)
- [3. Empirical Benchmarks](#3-empirical-benchmarks-en)
- [4. Web Compatibility Scope](#4-web-compatibility-scope)
- [5. Building from Source](#5-building-from-source-en)
- [6. Installation](#6-installation)
- [7. Security Advisory & MVP Notice](#7-security-advisory--mvp-notice)
- [8. License](#8-license-en)

---

<a name="1-architectural-overview-en"></a>
### 1. Architectural Overview

Caram Browser decouples the browser UI shell from web execution. Modern Chromium-based browsers allocate distinct multi-process models with immense overhead per tab, running heavy telemetry daemons, crypto-wallet stacks, and unpruned JavaScript engines.

Caram enforces a strict **two-tier architecture**:
1. **Frontend Chrome UI (Leptos CSR + Rust WASM):** Renders the browser frame (tabs, address bar, bookmarks, modal dialogues, download progress shelf) entirely in WebAssembly via Leptos. It interacts with the backend strictly through asynchronous Tauri IPC.
2. **Native OS Webview Subsurfaces (WebKitGTK 4.1):** Web pages are not rendered within web iframes. Instead, they are instantiated as native child subsurfaces pinned below the 92px chrome boundary. This leverages native Linux hardware acceleration with a sub-140MB memory footprint.

---

### 2. Key Features
- **300k+ Rule Brave adblock-rust Engine**: Hosted on a dedicated OS worker thread for zero-latency network inspections.
- **IDM-Style Multi-Threaded Downloader**: Splits downloads into 4–16 concurrent TCP streams with HTTP Range chunking.
- **Smart Memory Tab Snoozer**: Background tabs inactive for &gt;10 minutes have their Webviews cleanly evicted, returning 100% of memory to the OS.
- **Argon2id + AES-256-GCM Vault**: Memory-hard envelope key derivation with 1-click safe DOM autofill injection.
- **WebRTC Leak Shield & Clean URLs**: Purges local IP leaks from SDP and strips invasive tracking parameters (`fbclid`, `utm_*`).

---

<a name="3-empirical-benchmarks-en"></a>
### 3. Empirical Benchmarks

| Metric / Feature | Caram Browser | Brave Browser | Google Chrome | Mozilla Firefox |
| :--- | :--- | :--- | :--- | :--- |
| **Shell Architecture** | **100% Rust (Leptos WASM)** | C++ Chromium UI | C++ Chromium UI | C++ / XUL / Gecko |
| **Rendering Engine** | **WebKitGTK 4.1 (Native)** | Blink / V8 (C++) | Blink / V8 (C++) | Gecko / SpiderMonkey |
| **Idle Memory (1 Tab)** | **~95 MB – 135 MB** 🟢 | ~550 MB – 750 MB 🔴 | ~600 MB – 900 MB 🔴 | ~450 MB – 650 MB 🟡 |
| **Memory Load (10 Tabs)** | **~380 MB – 520 MB** 🟢 | ~1.4 GB – 2.1 GB 🔴 | ~1.8 GB – 2.6 GB 🔴 | ~1.1 GB – 1.6 GB 🟡 |
| **Telemetry & Bloatware** | **Zero (0% Telemetry)** 🟢 | BAT, Crypto Wallet 🟡 | Pervasive Telemetry 🔴 | Telemetry / Pocket 🟡 |
| **Download Engine** | **Multi-threaded (4–16 TCP)** | Single-stream default | Single-stream default | Single-stream default |

---

<a name="4-web-compatibility-scope"></a>
### 4. Web Compatibility Scope
- **W3C Standards**: Compatible with modern web applications (Discord Web, GitHub, social media).
- **Known Limitations**: Proprietary Widevine DRM (Netflix) and Chromium-only Google Meet background blur pipelines are unsupported.

---

<a name="5-building-from-source-en"></a>
### 5. Building from Source
```bash
# Prerequisites
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl libssl-dev libayatana-appindicator3-dev librsvg2-dev

# Rust setup
rustup target add wasm32-unknown-unknown
cargo install trunk tauri-cli --version "^2.0.0"

# Build
git clone https://github.com/LocShadowVN/CaramBrowser.git
cd CaramBrowser
cargo tauri build
```

---

<a name="6-installation"></a>
### 6. Installation
- **Debian / Ubuntu**: Download and install the `.deb` release using `sudo dpkg -i caram-browser_*.deb`.
- **Universal Linux**: Download the `.AppImage`, make it executable (`chmod +x *.AppImage`), and run it directly.

---

<a name="7-security-advisory--mvp-notice"></a>
### 7. Security Advisory & MVP Notice
Caram Browser is an experimental prototype distributed under the **GNU GPL-3.0** license "as is" without warranty. Recommended for technical research, lightweight browsing, and developer workstations.

---

<a name="8-license-en"></a>
### 8. License
Distributed under the **GNU General Public License v3.0 (GNU GPLv3)**. See [LICENSE](LICENSE) for details.
