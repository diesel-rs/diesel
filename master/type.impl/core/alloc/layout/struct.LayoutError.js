(function() {
    var type_impls = Object.fromEntries([["bumpalo",[]],["serde",[]],["serde_core",[]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[14,13,18]}