// generated by diplomat-tool
import { AbstractArgList } from "./AbstractArgList.mjs"
import { AbstractNodeId } from "./AbstractNodeId.mjs"
import { BuilderOpLike } from "./BuilderOpLike.mjs"
import { BuiltinQuery } from "./BuiltinQuery.mjs"
import { EdgeAbstract } from "./EdgeAbstract.mjs"
import { IntermediateState } from "./IntermediateState.mjs"
import { OpCtx } from "./OpCtx.mjs"
import { OperationBuilderError } from "./OperationBuilderError.mjs"
import { UserDefinedOperation } from "./UserDefinedOperation.mjs"
import wasm from "./diplomat-wasm.mjs";
import * as diplomatRuntime from "./diplomat-runtime.mjs";

const OperationBuilder_box_destroy_registry = new FinalizationRegistry((ptr) => {
    wasm.OperationBuilder_destroy(ptr);
});

export class OperationBuilder {
    // Internal ptr reference:
    #ptr = null;

    // Lifetimes are only to keep dependencies alive.
    // Since JS won't garbage collect until there are no incoming edges.
    #selfEdge = [];
    #aEdge = [];

    #internalConstructor(symbol, ptr, selfEdge, aEdge) {
        if (symbol !== diplomatRuntime.internalConstructor) {
            console.error("OperationBuilder is an Opaque type. You cannot call its constructor.");
            return;
        }
        this.#aEdge = aEdge;
        this.#ptr = ptr;
        this.#selfEdge = selfEdge;

        // Are we being borrowed? If not, we can register.
        if (this.#selfEdge.length === 0) {
            OperationBuilder_box_destroy_registry.register(this, this.#ptr);
        }

        return this;
    }
    /** @internal */
    get ffiValue() {
        return this.#ptr;
    }


    static create(opCtx, selfOpId) {
        // This lifetime edge depends on lifetimes 'a
        let aEdges = [opCtx];


        const result = wasm.OperationBuilder_create(opCtx.ffiValue, selfOpId);

        try {
            return new OperationBuilder(diplomatRuntime.internalConstructor, result, [], aEdges);
        }

        finally {
        }
    }

    expectParameterNode(marker) {
        let functionCleanupArena = new diplomatRuntime.CleanupArena();

        const markerSlice = diplomatRuntime.DiplomatBuf.str8(wasm, marker);
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_expect_parameter_node(diplomatReceive.buffer, this.ffiValue, ...markerSlice.splat());

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            functionCleanupArena.free();

            diplomatReceive.free();
        }
    }

    expectContextNode(marker) {
        let functionCleanupArena = new diplomatRuntime.CleanupArena();

        const markerSlice = diplomatRuntime.DiplomatBuf.str8(wasm, marker);
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_expect_context_node(diplomatReceive.buffer, this.ffiValue, ...markerSlice.splat());

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            functionCleanupArena.free();

            diplomatReceive.free();
        }
    }

    expectParameterEdge(src, dst, av) {
        let functionCleanupArena = new diplomatRuntime.CleanupArena();

        const srcSlice = diplomatRuntime.DiplomatBuf.str8(wasm, src);
        const dstSlice = diplomatRuntime.DiplomatBuf.str8(wasm, dst);
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_expect_parameter_edge(diplomatReceive.buffer, this.ffiValue, ...srcSlice.splat(), ...dstSlice.splat(), av.ffiValue);

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            functionCleanupArena.free();

            diplomatReceive.free();
        }
    }

    startQuery(query, args) {
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_start_query(diplomatReceive.buffer, this.ffiValue, query.ffiValue, args.ffiValue);

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            diplomatReceive.free();
        }
    }

    enterTrueBranch() {
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_enter_true_branch(diplomatReceive.buffer, this.ffiValue);

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            diplomatReceive.free();
        }
    }

    enterFalseBranch() {
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_enter_false_branch(diplomatReceive.buffer, this.ffiValue);

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            diplomatReceive.free();
        }
    }

    startShapeQuery(opMarker) {
        let functionCleanupArena = new diplomatRuntime.CleanupArena();

        const opMarkerSlice = diplomatRuntime.DiplomatBuf.str8(wasm, opMarker);
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_start_shape_query(diplomatReceive.buffer, this.ffiValue, ...opMarkerSlice.splat());

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            functionCleanupArena.free();

            diplomatReceive.free();
        }
    }

    endQuery() {
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_end_query(diplomatReceive.buffer, this.ffiValue);

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            diplomatReceive.free();
        }
    }

    expectShapeNode(nodeName) {
        let functionCleanupArena = new diplomatRuntime.CleanupArena();

        const nodeNameSlice = diplomatRuntime.DiplomatBuf.str8(wasm, nodeName);
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_expect_shape_node(diplomatReceive.buffer, this.ffiValue, ...nodeNameSlice.splat());

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            functionCleanupArena.free();

            diplomatReceive.free();
        }
    }

    expectShapeEdge(src, dst, av) {
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_expect_shape_edge(diplomatReceive.buffer, this.ffiValue, src.ffiValue, dst.ffiValue, av.ffiValue);

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            diplomatReceive.free();
        }
    }

    addOperation(name, instruction, args) {
        let functionCleanupArena = new diplomatRuntime.CleanupArena();

        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_add_operation(diplomatReceive.buffer, this.ffiValue, ...diplomatRuntime.optionToArgsForCalling(name, 8, 4, (arrayBuffer, offset, jsValue) => [functionCleanupArena.alloc(diplomatRuntime.DiplomatBuf.str8(wasm, jsValue)).writePtrLenToArrayBuffer(arrayBuffer, offset + 0)]), instruction.ffiValue, args.ffiValue);

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            functionCleanupArena.free();

            diplomatReceive.free();
        }
    }

    renameNode(aid, newName) {
        let functionCleanupArena = new diplomatRuntime.CleanupArena();

        const newNameSlice = diplomatRuntime.DiplomatBuf.str8(wasm, newName);
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_rename_node(diplomatReceive.buffer, this.ffiValue, aid.ffiValue, ...newNameSlice.splat());

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
        }

        finally {
            functionCleanupArena.free();

            diplomatReceive.free();
        }
    }

    show() {
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_show(diplomatReceive.buffer, this.ffiValue);

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
            return new IntermediateState(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
        }

        finally {
            diplomatReceive.free();
        }
    }

    finalize() {
        const diplomatReceive = new diplomatRuntime.DiplomatReceiveBuf(wasm, 5, 4, true);


        const result = wasm.OperationBuilder_finalize(diplomatReceive.buffer, this.ffiValue);

        try {
            if (!diplomatReceive.resultFlag) {
                const cause = new OperationBuilderError(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
                throw new globalThis.Error('OperationBuilderError: ' + cause.toString(), { cause });
            }
            return new UserDefinedOperation(diplomatRuntime.internalConstructor, diplomatRuntime.ptrRead(wasm, diplomatReceive.buffer), []);
        }

        finally {
            diplomatReceive.free();
        }
    }

    constructor(symbol, ptr, selfEdge, aEdge) {
        return this.#internalConstructor(...arguments)
    }
}