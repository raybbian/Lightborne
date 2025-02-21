(function() {
    var type_impls = Object.fromEntries([["alsa_sys",[]],["ash",[]],["khronos_egl",[]],["renderdoc_sys",[]],["wgpu_core",[]],["winit",[]],["x11_dl",[]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[15,11,19,21,17,13,14]}