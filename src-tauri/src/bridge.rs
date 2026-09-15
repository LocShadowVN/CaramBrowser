pub const CHROME_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

pub fn get_webbridge_script() -> &'static str {
    r#"
    (function() {
        'use strict';

        // ============================================================
        // 1. NAVIGATOR & CLIENT HINTS SPOOFING (CHROME 130 ON LINUX)
        // ============================================================
        try {
            const CHROME_UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";
            
            Object.defineProperty(navigator, 'userAgent', { get: () => CHROME_UA, configurable: true });
            Object.defineProperty(navigator, 'appVersion', { get: () => CHROME_UA.replace('Mozilla/', ''), configurable: true });
            Object.defineProperty(navigator, 'platform', { get: () => 'Linux x86_64', configurable: true });
            Object.defineProperty(navigator, 'vendor', { get: () => 'Google Inc.', configurable: true });
            Object.defineProperty(navigator, 'vendorSub', { get: () => '', configurable: true });
            Object.defineProperty(navigator, 'productSub', { get: () => '20030107', configurable: true });

            // Polyfill Client Hints (navigator.userAgentData) cho Google Meet / Discord
            const uaData = {
                brands: [
                    { brand: 'Google Chrome', version: '130' },
                    { brand: 'Chromium', version: '130' },
                    { brand: 'Not?A_Brand', version: '24' }
                ],
                mobile: false,
                platform: 'Linux',
                getHighEntropyValues: function(hints) {
                    return Promise.resolve({
                        architecture: 'x86',
                        bitness: '64',
                        brands: this.brands,
                        fullVersionList: [
                            { brand: 'Google Chrome', version: '130.0.6723.116' },
                            { brand: 'Chromium', version: '130.0.6723.116' },
                            { brand: 'Not?A_Brand', version: '24.0.0.0' }
                        ],
                        mobile: false,
                        model: '',
                        platform: 'Linux',
                        platformVersion: '6.5.0',
                        uaFullVersion: '130.0.6723.116'
                    });
                },
                toJSON: function() {
                    return { brands: this.brands, mobile: this.mobile, platform: this.platform };
                }
            };
            Object.defineProperty(navigator, 'userAgentData', { get: () => uaData, configurable: true });
        } catch (e) {}

        // ============================================================
        // 2. WINDOW.CHROME RUNTIME & INTERNAL METRICS POLYFILL
        // ============================================================
        try {
            if (!window.chrome) {
                window.chrome = {};
            }

            window.chrome.app = {
                isInstalled: false,
                InstallState: { DISABLED: 'disabled', INSTALLED: 'installed', NOT_INSTALLED: 'not_installed' },
                RunningState: { CANNOT_RUN: 'cannot_run', READY_TO_RUN: 'ready_to_run', RUNNING: 'running' },
                getDetails: function() { return null; },
                getIsInstalled: function() { return false; },
                runningState: function() { return 'cannot_run'; }
            };

            window.chrome.csi = function() {
                const now = performance.now();
                return { startE: now, onloadT: now, pageT: now, tran: 15 };
            };

            window.chrome.loadTimes = function() {
                const nowSec = performance.now() / 1000;
                return {
                    requestTime: nowSec,
                    startLoadTime: nowSec,
                    commitLoadTime: nowSec,
                    finishDocumentLoadTime: nowSec,
                    finishLoadTime: nowSec,
                    firstPaintTime: nowSec,
                    firstPaintAfterLoadTime: 0,
                    navigationType: 'Other',
                    wasFetchedViaSpdy: true,
                    wasNpnNegotiated: true,
                    npnNegotiatedProtocol: 'h2',
                    wasAlternateProtocolAvailable: false,
                    connectionInfo: 'h2'
                };
            };

            window.chrome.runtime = {
                id: 'caram-webbridge-runtime',
                connect: function() {
                    return {
                        onMessage: { addListener: function() {}, removeListener: function() {} },
                        onDisconnect: { addListener: function() {}, removeListener: function() {} },
                        postMessage: function() {}
                    };
                },
                sendMessage: function(extensionId, message, options, responseCallback) {
                    const cb = typeof options === 'function' ? options : responseCallback;
                    if (cb) setTimeout(() => cb({ success: true }), 0);
                },
                getManifest: function() {
                    return { name: 'Caram WebBridge', version: '1.0.0', manifest_version: 3 };
                }
            };
        } catch (e) {}

        // ============================================================
        // 3. WEBRTC & MEDIADEVICES CONSTRAINTS SHIM (MEET / DISCORD)
        // ============================================================
        try {
            if (window.MediaStreamTrack && !MediaStreamTrack.prototype.getCapabilities) {
                MediaStreamTrack.prototype.getCapabilities = function() {
                    return {
                        aspectRatio: { max: 1920, min: 0.00052 },
                        facingMode: ['user', 'environment'],
                        frameRate: { max: 60, min: 1 },
                        height: { max: 1080, min: 1 },
                        width: { max: 1920, min: 1 },
                        deviceId: 'default',
                        groupId: 'default'
                    };
                };
            }

            if (window.RTCRtpSender && !RTCRtpSender.getCapabilities) {
                RTCRtpSender.getCapabilities = function(kind) {
                    if (kind === 'video') {
                        return {
                            codecs: [
                                { mimeType: 'video/VP8', clockRate: 90000 },
                                { mimeType: 'video/H264', clockRate: 90000 },
                                { mimeType: 'video/VP9', clockRate: 90000 }
                            ],
                            headerExtensions: []
                        };
                    }
                    return {
                        codecs: [
                            { mimeType: 'audio/opus', clockRate: 48000, channels: 2 }
                        ],
                        headerExtensions: []
                    };
                };
            }

            // Permissions query guard: Tránh văng ngoại lệ khi web hỏi các quyền lạ
            if (navigator.permissions && navigator.permissions.query) {
                const origQuery = navigator.permissions.query.bind(navigator.permissions);
                navigator.permissions.query = function(param) {
                    return origQuery(param).catch(() => {
                        return Promise.resolve({ state: 'prompt', onchange: null });
                    });
                };
            }

            // Screen Orientation Mock
            if (window.screen && !window.screen.orientation) {
                window.screen.orientation = {
                    type: 'landscape-primary',
                    angle: 0,
                    lock: () => Promise.resolve(),
                    unlock: () => {}
                };
            }
        } catch (e) {}
    })();
    "#
}
