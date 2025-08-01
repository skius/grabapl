// generated by diplomat-tool
import wasm from "./diplomat-wasm.mjs";
import * as diplomatRuntime from "./diplomat-runtime.mjs";

const IntermediateState_box_destroy_registry = new FinalizationRegistry((ptr) => {
    wasm.IntermediateState_destroy(ptr);
});

export class IntermediateState {
    // Internal ptr reference:
    #ptr = null;

    // Lifetimes are only to keep dependencies alive.
    // Since JS won't garbage collect until there are no incoming edges.
    #selfEdge = [];

    #internalConstructor(symbol, ptr, selfEdge) {
        if (symbol !== diplomatRuntime.internalConstructor) {
            console.error("IntermediateState is an Opaque type. You cannot call its constructor.");
            return;
        }
        this.#ptr = ptr;
        this.#selfEdge = selfEdge;

        // Are we being borrowed? If not, we can register.
        if (this.#selfEdge.length === 0) {
            IntermediateState_box_destroy_registry.register(this, this.#ptr);
        }

        return this;
    }
    /** @internal */
    get ffiValue() {
        return this.#ptr;
    }


    getDot() {
        const write = new diplomatRuntime.DiplomatWriteBuf(wasm);

    wasm.IntermediateState_get_dot(this.ffiValue, write.buffer);

        try {
            return write.readString8();
        }

        finally {
            write.free();
        }
    }

    availableAids() {
        const write = new diplomatRuntime.DiplomatWriteBuf(wasm);

    wasm.IntermediateState_available_aids(this.ffiValue, write.buffer);

        try {
            return write.readString8();
        }

        finally {
            write.free();
        }
    }

    queryContext() {
        const write = new diplomatRuntime.DiplomatWriteBuf(wasm);

    wasm.IntermediateState_query_context(this.ffiValue, write.buffer);

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