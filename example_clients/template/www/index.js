import {ConcreteGraph, Grabapl} from 'template-ffi'

Grabapl.init();

let src = 'fn fail() { diverge<"failing on purpose">(); }';

let res = Grabapl.parse(src);
let program;
try {
    program = res.getProgram();

    try {
        let inputGraph = ConcreteGraph.create();
        program.runOperation(inputGraph, "fail", []);
    } catch (e) {
        // To get the raw error message:
        let causeStr = e.cause ? e.cause.toString() : e.toString();
        // (but string interpolation works as well with a bit of fluff)
        let message = `Error running operation: ${causeStr}, e: ${e}`;
        console.error(message);
    }
} catch (e) {
    let causeStr = e.cause ? e.cause.toString() : e.toString();
    let message = `Error parsing program: ${causeStr}`;
    console.error(message);
}

