import './restart-audio-context.js'
import init from './bevy_game.js'

init().catch((error) => {
    if (!error.message.startsWith("Using exceptions for control flow, don't mind me. This isn't actually an error!")) {
        throw error;
    }
});

const canvas = document.getElementById("bevy-container");
const infoLeft = document.getElementById("info-panel-left");
const infoRight = document.getElementById("info-panel-right");
const gl = canvas.getContext("webgl2");
if (gl) {
    const debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
    if (debugInfo) {
        const vendor = gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL);
        const renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL);

        infoLeft.textContent += `Vendor: ${vendor}, Renderer: ${renderer}`

        if (/intel/i.test(vendor)) {
            infoRight.textContent += 'Likely using an integrated GPU. Running slow? Make sure hardware acceleration is on.';
        } else if (/nvidia|amd/i.test(vendor)) {
            infoRight.textContent += 'Likely using a dedicated GPU.';
        } else {
            infoRight.textContent += 'Could not determine GPU type. Running slow? Make sure hardware acceleration is on.';
        }
    } else {
        infoRight.textContent += 'WebGL debug renderer info is not available.';
    }
} else {
    infoRight.textContent += 'WebGL is not supported on your browser.';
}
