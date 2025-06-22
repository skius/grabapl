import {
    AbstractArgList,
    AbstractNodeId, BuilderOpLike, BuiltinQuery,
    EdgeAbstract,
    OpCtx,
    OperationBuilder,
    OperationBuilderError
} from "simple-semantics";
import {Graphviz} from "@hpcc-js/wasm";

let opCtx = OpCtx.create();

export const getOpCtx = () => {
    return opCtx;
}

let op_builder = OperationBuilder.create(opCtx);

const svgContainer = document.querySelector("#opb-svgContainer");
const infoContainer  = document.querySelector("#opb-info");
const graphviz = await Graphviz.load();

const updateState = () => {
    let intermediate_state = op_builder.show();
    let dot = intermediate_state.getDot();

    let svg = graphviz.dot(dot);
    svgContainer.innerHTML = svg;


    // update mapping
    let availableAids = intermediate_state.availableAids();
    let queryContext = intermediate_state.queryContext();
    let desiredText = `${availableAids}\n${queryContext}`;

    let textElement = document.createTextNode(desiredText);
    infoContainer.innerHTML = ""; // Clear previous content
    infoContainer.appendChild(textElement);
}

const initCommands = () => {
    let commands = [
        {
            "name": "Expect Parameter Node",
            "inputs": ["SubstMarker"],
            "invoke": (num_str) => {
                let num = parseInt(num_str);
                op_builder.expectParameterNode(num);
            }
        },
        {
            "name": "Expect Parameter Edge",
            "inputs": ["Source", "Target", "Abstract Value"],
            "invoke": (src, target, av) => {
                let src_id = parseInt(src);
                let target_id = parseInt(target);
                let abstract_value = av === "*" ? EdgeAbstract.newWildcard() : EdgeAbstract.newExact(av);
                console.log(`Expecting edge from ${src_id} to ${target_id} with abstract value ${abstract_value}`);
                op_builder.expectParameterEdge(src_id, target_id, abstract_value);
            }
        },

        {
            "name": "Expect Context Node",
            "inputs": ["SubstMarker"],
            "invoke": (num_str) => {
                let num = parseInt(num_str);
                op_builder.expectContextNode(num);
            }
        },
        {
            "name": "Start IsValueGt Query",
            "inputs": ["Node", "Value"],
            "invoke": (node_aid_str, val) => {
                let value = parseInt(val);
                let args = AbstractArgList.create();
                args.push(AbstractNodeId.newFromStr(node_aid_str))
                let query = BuiltinQuery.newIsValueGt(value);
                op_builder.startQuery(query, args);
            }
        },
        {
            "name": "Start IsValueEq Query",
            "inputs": ["Node", "Value"],
            "invoke": (node_aid_str, val) => {
                let value = parseInt(val);
                let args = AbstractArgList.create();
                args.push(AbstractNodeId.newFromStr(node_aid_str))
                let query = BuiltinQuery.newIsValueEq(value);
                op_builder.startQuery(query, args);
            }
        },
        {
            "name": "Start ValuesEqual Query",
            "inputs": ["Node", "Node"],
            "invoke": (node_a_str, node_b_str) => {
                let args = AbstractArgList.create();
                args.push(AbstractNodeId.newFromStr(node_a_str));
                args.push(AbstractNodeId.newFromStr(node_b_str));
                let query = BuiltinQuery.newValuesEqual();
                op_builder.startQuery(query, args);
            }
        },
        {
            "name": "Start FirstGtSnd Query",
            "inputs": ["Node", "Node"],
            "invoke": (node_a_str, node_b_str) => {
                let args = AbstractArgList.create();
                args.push(AbstractNodeId.newFromStr(node_a_str));
                args.push(AbstractNodeId.newFromStr(node_b_str));
                let query = BuiltinQuery.newFirstGtSnd();
                op_builder.startQuery(query, args);
            }
        },
        {
            "name": "Enter True Branch",
            "inputs": [],
            "invoke": () => {
                op_builder.enterTrueBranch();
            }
        },
        {
            "name": "Enter False Branch",
            "inputs": [],
            "invoke": () => {
                op_builder.enterFalseBranch();
            }
        },
        {
            "name": "Start Shape Query",
            "inputs": ["Query Name"],
            "invoke": (name) => {
                op_builder.startShapeQuery(name);
            }
        },
        {
            "name": "Expect Shape Node",
            "inputs": ["Node Name"],
            "invoke": (name) => {
                op_builder.expectShapeNode(name);
            }
        },
        {
            "name": "Expect Shape Edge",
            "inputs": ["From Node", "To Node", "Edge Value"],
            "invoke": (from, to, av) => {
                let from_id = AbstractNodeId.newFromStr(from);
                let to_id = AbstractNodeId.newFromStr(to);
                let abstract_value = av === "*" ? EdgeAbstract.newWildcard() : EdgeAbstract.newExact(av);
                op_builder.expectShapeEdge(from_id, to_id, abstract_value);
            }
        },
        {
            "name": "End Query",
            "inputs": [],
            "invoke": () => {
                op_builder.endQuery();
            }
        },
        {
            "name": "Add Node",
            "inputs": ["Operation Name"],
            "invoke": (op_marker) => {
                let op = BuilderOpLike.newAddNode();
                let args = AbstractArgList.create();
                op_builder.addInstruction(op_marker, op, args);
            }
        },
        {
            "name": "Sample",
            "inputs": [],
            "invoke": (input1) => {
            }
        },
    ]


    // add all of these commands with the appropriate number of inputs, each labeled according to the array, to the
    // div #opb-commands

    let commandDiv = document.querySelector("#opb-commands");
    commands.forEach((command, command_index) => {
        let handle_command = () => {
            let inputs = [];
            command.inputs.forEach((input, index) => {
                let inputElement = document.querySelector(`#opb-input-${command_index}-${index}`);
                if (inputElement) {
                    inputs.push(inputElement.value);
                    inputElement.value = "";
                }
            });
            try {
                console.log(`Invoking command: ${command.name} with inputs: ${inputs}`);
                command.invoke(...inputs);
            } catch (e) {
                // get cause of e
                let cause = e.cause ? e.cause : e;
                if (cause instanceof OperationBuilderError) {
                    console.error(cause.message());
                } else {
                    console.error("An unexpected error occurred:", e);
                }
            }
            updateState();

        }
        let button_and_inputs = document.createElement("div");
        button_and_inputs.className = "opb-command";
        let label = document.createElement("label");
        label.innerText = command.name;
        button_and_inputs.appendChild(label);
        command.inputs.forEach((input, index) => {
            let inputElement = document.createElement("input");
            inputElement.type = "text";
            inputElement.id = `opb-input-${command_index}-${index}`;
            inputElement.placeholder = input;

            // submit on enter
            inputElement.addEventListener("keydown", (event) => {
                if (event.key === "Enter") {
                    event.preventDefault();
                    handle_command();
                }
            });

            button_and_inputs.appendChild(inputElement);
        });
        let button = document.createElement("button");
        button.innerText = command.name;
        button.addEventListener("click", () => {
            handle_command();
        });
        button_and_inputs.appendChild(button);
        commandDiv.appendChild(button_and_inputs);
    });
}

initCommands();

let finalizeButton = document.querySelector("#opb-btnFinalize");
let opIdInput = document.querySelector("#opb-opId");
finalizeButton.addEventListener("click", () => {
    let opId = parseInt(opIdInput.value);
    let op;
    try {
        op = op_builder.finalize(opId);
    } catch (e) {
        // get cause of e
        let cause = e.cause ? e.cause : e;
        if (cause instanceof OperationBuilderError) {
            console.error(cause.message());
        } else {
            console.error("An unexpected error occurred:", e);
        }
        return; // exit early if there was an error
    }
    // move out of op_builder for safety and soundness reasons TODO: check does this even help?
    op_builder = null
    opCtx.addCustomOperation(opId, op);
    // recreate
    op_builder = OperationBuilder.create(opCtx);
    updateState();

    console.log(`Finalized operation with id ${opId}`);
})