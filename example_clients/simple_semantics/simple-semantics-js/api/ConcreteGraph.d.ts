// generated by diplomat-tool
import type { pointer, codepoint } from "./diplomat-runtime.d.ts";



export class ConcreteGraph {
    /** @internal */
    get ffiValue(): pointer;
    /** @internal */
    constructor();


    static create(): ConcreteGraph;

    addNode(value: number): number;

    addEdge(from: number, to: number, value: string): void;

    sayHi(): void;
}