// generated by diplomat-tool
import type { ParseResult } from "./ParseResult"
import type { pointer, codepoint } from "./diplomat-runtime.d.ts";

export type Context_obj = {
    i: number;
};



export class Context {
    get i(): number;
    set i(value: number);
    /** @internal */
    static fromFields(structObj : Context_obj) : Context;

    /**
    * Create `Context` from an object that contains all of `Context`s fields.
    * Optional fields do not need to be included in the provided object.
    */
    constructor(structObj: Context_obj);


    static init(): void;

    static parse(src: string): ParseResult;
}