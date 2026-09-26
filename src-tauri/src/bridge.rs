pub const CHROME_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

pub fn get_webbridge_script() -> &'static str {
    r#"
    (function() {
        'use strict';

        try {
            if (window.RTCPeerConnection) {
                const OrigPeerConnection = window.RTCPeerConnection;

                function sanitizeCandidate(candStr) {
                    if (!candStr || typeof candStr !== 'string') return candStr;
                    const privateIpRegex = /(?:192\.168\.\d{1,3}\.\d{1,3}|10\.\d{1,3}\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[0-1])\.\d{1,3}\.\d{1,3}|fe80::[0-9a-fA-F:]+)/;
                    if (candStr.includes('typ host') && privateIpRegex.test(candStr)) {
                        return null;
                    }
                    return candStr;
                }

                function sanitizeSdp(sdpStr) {
                    if (!sdpStr || typeof sdpStr !== 'string') return sdpStr;
                    return sdpStr.split('\r\n').filter(line => {
                        if (line.startsWith('a=candidate:') && line.includes('typ host')) {
                            const privateIpRegex = /(?:192\.168\.\d{1,3}\.\d{1,3}|10\.\d{1,3}\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[0-1])\.\d{1,3}\.\d{1,3})/;
                            return !privateIpRegex.test(line);
                        }
                        return true;
                    }).join('\r\n');
                }

                window.RTCPeerConnection = function(config, constraints) {
                    const pc = new OrigPeerConnection(config, constraints);

                    const origCreateOffer = pc.createOffer.bind(pc);
                    pc.createOffer = function(options) {
                        return origCreateOffer(options).then(offer => {
                            offer.sdp = sanitizeSdp(offer.sdp);
                            return offer;
                        });
                    };

                    const origCreateAnswer = pc.createAnswer.bind(pc);
                    pc.createAnswer = function(options) {
                        return origCreateAnswer(options).then(answer => {
                            answer.sdp = sanitizeSdp(answer.sdp);
                            return answer;
                        });
                    };

                    let userIceHandler = null;
                    Object.defineProperty(pc, 'onicecandidate', {
                        set: function(fn) {
                            userIceHandler = fn;
                            pc.addEventListener('icecandidate', function(e) {
                                if (e.candidate && e.candidate.candidate) {
                                    const sanitized = sanitizeCandidate(e.candidate.candidate);
                                    if (!sanitized) {
                                        e.stopImmediatePropagation();
                                        return;
                                    }
                                }
                                if (typeof userIceHandler === 'function') {
                                    userIceHandler.apply(this, arguments);
                                }
                            });
                        },
                        get: function() { return userIceHandler; }
                    });

                    return pc;
                };

                window.RTCPeerConnection.prototype = OrigPeerConnection.prototype;
            }
        } catch (e) {}

        try {
            const CHROME_UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

            Object.defineProperty(navigator, 'userAgent', { get: () => CHROME_UA, configurable: true });
            Object.defineProperty(navigator, 'appVersion', { get: () => CHROME_UA.replace('Mozilla/', ''), configurable: true });
            Object.defineProperty(navigator, 'platform', { get: () => 'Linux x86_64', configurable: true });
            Object.defineProperty(navigator, 'vendor', { get: () => 'Google Inc.', configurable: true });
            Object.defineProperty(navigator, 'vendorSub', { get: () => '', configurable: true });
            Object.defineProperty(navigator, 'productSub', { get: () => '20030107', configurable: true });

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

        try {
            if (!window.chrome) window.chrome = {};

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
                id: 'vibird-webbridge-runtime',
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
                    return { name: 'Vibird WebBridge', version: '1.0.0', manifest_version: 3 };
                }
            };
        } catch (e) {}

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
                        codecs: [{ mimeType: 'audio/opus', clockRate: 48000, channels: 2 }],
                        headerExtensions: []
                    };
                };
            }

            if (navigator.permissions && navigator.permissions.query) {
                const origQuery = navigator.permissions.query.bind(navigator.permissions);
                navigator.permissions.query = function(param) {
                    return origQuery(param).catch(() => {
                        return Promise.resolve({ state: 'prompt', onchange: null });
                    });
                };
            }

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

pub fn get_autofill_script() -> &'static str {
    r#"
    (function() {
        'use strict';
        if (window.__VIBIRD_AUTOFILL) return;

        function setNativeValue(el, value) {
            if (!el) return false;
            try {
                const proto = Object.getPrototypeOf(el);
                const desc = Object.getOwnPropertyDescriptor(proto, 'value');
                if (desc && desc.set) {
                    desc.set.call(el, value);
                } else {
                    el.value = value;
                }
                el.dispatchEvent(new Event('input', { bubbles: true }));
                el.dispatchEvent(new Event('change', { bubbles: true }));
                return true;
            } catch (e) {
                return false;
            }
        }

        window.__VIBIRD_AUTOFILL = function(user, pass) {
            const pw = document.querySelector('input[type=password]:not([disabled]):not([readonly])');
            if (!pw) return false;
            setNativeValue(pw, pass);
            const form = pw.form || pw.closest('form') || document;
            const candidates = form.querySelectorAll(
                'input[type=text]:not([disabled]), input[type=email]:not([disabled]), input[name*=user i], input[name*=login i], input[name*=email i], input[autocomplete=username]'
            );
            for (let i = 0; i < candidates.length; i++) {
                const el = candidates[i];
                if (el.offsetParent !== null || el.getClientRects().length > 0) {
                    setNativeValue(el, user);
                    break;
                }
            }
            return true;
        };
    })();
    "#
}
