import * as wasm from "webrtc-transport";

const originalLog = console.log;

console.log = (...args) => {
    //alert(args);
    originalLog(...args);
}

const originalError = console.error;

console.error = (...args) => {
    //alert(...args);
    originalError(...args);
}

wasm.start()
