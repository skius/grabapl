// generated by diplomat-tool
import type { pointer, codepoint } from "./diplomat-runtime.d.ts";



export class StringWrapper {
    /** @internal */
    get ffiValue(): pointer;
    /** @internal */
    constructor();


    static new_(s: string): StringWrapper;

    toString(): string;
}