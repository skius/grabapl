// generated by diplomat-tool
import type { OperationContext } from "./OperationContext"
import type { StringError } from "./StringError"
import type { pointer, codepoint } from "./diplomat-runtime.d.ts";



/**
 * A user defined operation that is currently being built using low-level instructions instead of
 * parsing via the syntax parser.
 *
 * This builder should probably be used to create an interactive interface for building user defined operations.
 */
export class OperationBuilder {
    /** @internal */
    get ffiValue(): pointer;
    /** @internal */
    constructor();


    /**
     * Creates a new operation builder for the given operation context and with the given self operation ID.
     *
     * The passed operation context holds the other user defined operations that can be used in the builder.
     */
    static create(opCtx: OperationContext, selfOpId: number): OperationBuilder;

    /**
     * Adds an expected parameter node with the given name and type to the operation.
     */
    expectParameterNode(name: string, nodeType: string): void;
}