// generated by diplomat-tool
import wasm from "./diplomat-wasm.mjs";
import * as diplomatRuntime from "./diplomat-runtime.mjs";

const UserDefinedOperation_box_destroy_registry = new FinalizationRegistry((ptr) => {
    wasm.UserDefinedOperation_destroy(ptr);
});

export class UserDefinedOperation {
    // Internal ptr reference:
    #ptr = null;

    // Lifetimes are only to keep dependencies alive.
    // Since JS won't garbage collect until there are no incoming edges.
    #selfEdge = [];

    #internalConstructor(symbol, ptr, selfEdge) {
        if (symbol !== diplomatRuntime.internalConstructor) {
            console.error("UserDefinedOperation is an Opaque type. You cannot call its constructor.");
            return;
        }
        this.#ptr = ptr;
        this.#selfEdge = selfEdge;

        // Are we being borrowed? If not, we can register.
        if (this.#selfEdge.length === 0) {
            UserDefinedOperation_box_destroy_registry.register(this, this.#ptr);
        }

        return this;
    }
    /** @internal */
    get ffiValue() {
        return this.#ptr;
    }


    constructor(symbol, ptr, selfEdge) {
        return this.#internalConstructor(...arguments)
    }
}