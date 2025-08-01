// generated by diplomat-tool
import wasm from "./diplomat-wasm.mjs";
import * as diplomatRuntime from "./diplomat-runtime.mjs";

const StringError_box_destroy_registry = new FinalizationRegistry((ptr) => {
    wasm.StringError_destroy(ptr);
});

/**
 * Catch this in a try-catch and print it with toString().
 */
export class StringError {
    // Internal ptr reference:
    #ptr = null;

    // Lifetimes are only to keep dependencies alive.
    // Since JS won't garbage collect until there are no incoming edges.
    #selfEdge = [];

    #internalConstructor(symbol, ptr, selfEdge) {
        if (symbol !== diplomatRuntime.internalConstructor) {
            console.error("StringError is an Opaque type. You cannot call its constructor.");
            return;
        }
        this.#ptr = ptr;
        this.#selfEdge = selfEdge;

        // Are we being borrowed? If not, we can register.
        if (this.#selfEdge.length === 0) {
            StringError_box_destroy_registry.register(this, this.#ptr);
        }

        return this;
    }
    /** @internal */
    get ffiValue() {
        return this.#ptr;
    }


    toString() {
        const write = new diplomatRuntime.DiplomatWriteBuf(wasm);

    wasm.StringError_to_string(this.ffiValue, write.buffer);

        try {
            return write.readString8();
        }

        finally {
            write.free();
        }
    }

    constructor(symbol, ptr, selfEdge) {
        return this.#internalConstructor(...arguments)
    }
}