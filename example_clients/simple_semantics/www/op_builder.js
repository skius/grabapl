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

let opIdInput = document.querySelector("#opb-opId");
let nextOpId = 1000;
opIdInput.value = nextOpId;

const getNextOpId = () => {
    let id = nextOpId;
    opIdInput.value = nextOpId;
    nextOpId += 1;
    return id;
}

let op_builder = OperationBuilder.create(opCtx, getNextOpId());

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
            "invoke": (p) => {
                op_builder.expectParameterNode(p);
            }
        },
        {
            "name": "Expect Parameter Edge",
            "inputs": ["Source", "Target", "Abstract Value"],
            "invoke": (src_id, target_id, av) => {
                let abstract_value = av === "*" ? EdgeAbstract.newWildcard() : EdgeAbstract.newExact(av);
                console.log(`Expecting edge from ${src_id} to ${target_id} with abstract value ${abstract_value}`);
                op_builder.expectParameterEdge(src_id, target_id, abstract_value);
            }
        },

        {
            "name": "Expect Context Node",
            "inputs": ["SubstMarker"],
            "invoke": (c) => {
                op_builder.expectContextNode(c);
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
            "name": "Rename Node",
            "inputs": ["Old AID", "New Name"],
            "invoke": (old_aid_str, new_name) => {
                let old_aid = AbstractNodeId.newFromStr(old_aid_str);
                op_builder.renameNode(old_aid, new_name);
            }
        },
        {
            "name": "Add Node",
            "inputs": ["Operation Name"],
            "invoke": (op_marker) => {
                let op = BuilderOpLike.newAddNode();
                let args = AbstractArgList.create();
                op_builder.addOperation(op_marker, op, args);
            }
        },
        {
            "name": "Add Edge",
            "inputs": ["From", "To"],
            "invoke": (from, to) => {
                let from_id = AbstractNodeId.newFromStr(from);
                let to_id = AbstractNodeId.newFromStr(to);
                let op = BuilderOpLike.newAddEdge();
                let args = AbstractArgList.create();
                args.push(from_id);
                args.push(to_id);
                op_builder.addOperation(null, op, args);
            }
        },
        {
            "name": "Set Edge Value",
            "inputs": ["From", "To", "Value"],
            "invoke": (from, to, exact_value) => {
                let from_id = AbstractNodeId.newFromStr(from);
                let to_id = AbstractNodeId.newFromStr(to);
                let op = BuilderOpLike.newSetEdgeValue(exact_value);
                let args = AbstractArgList.create();
                args.push(from_id);
                args.push(to_id);
                op_builder.addOperation(null, op, args);
            }
        },
        {
            "name": "Run Operation Id",
            "inputs": ["Output Name", "Operation Id", "Input1 + Input2 + ..."],
            "invoke": (name, opId, str_inputs) => {
                let op_id = parseInt(opId);
                // parse inputs
                let input_list = str_inputs.split("+").map(input => input.trim());
                // remove empty strings
                input_list = input_list.filter(input => input !== "");
                let args = AbstractArgList.create();
                input_list.forEach(input => {
                    let node_id = AbstractNodeId.newFromStr(input);
                    args.push(node_id);
                });
                console.log(`Running operation with id ${op_id} and inputs: ${input_list}`);
                let op = BuilderOpLike.newFromId(op_id);
                op_builder.addOperation(name, op, args);
            }
        },
        {
            "name": "Recurse",
            "inputs": ["Output Name", "Input1 + Input2 + ..."],
            "invoke": (name, str_inputs) => {
                // parse inputs
                let input_list = str_inputs.split("+").map(input => input.trim());
                // remove empty strings
                input_list = input_list.filter(input => input !== "");
                let args = AbstractArgList.create();
                input_list.forEach(input => {
                    let node_id = AbstractNodeId.newFromStr(input);
                    args.push(node_id);
                });
                console.log(`recursing with inputs: ${input_list}`);
                let op = BuilderOpLike.newRecurse();
                op_builder.addOperation(name, op, args);
            }
        },
        {
            "name": "To JSON",
            "inputs": [],
            "invoke": () => {
                let custom_op = op_builder.finalize();
                let new_op_ctx = OpCtx.create();
                new_op_ctx.addCustomOperation(1111, custom_op);
                let json = new_op_ctx.customOpToJson(1111);
                console.log("Custom operation JSON:\n", json);
            }
        },
        {
            "name": "To Base64",
            "inputs": [],
            "invoke": () => {
                let custom_op = op_builder.finalize();
                let new_op_ctx = OpCtx.create();
                new_op_ctx.addCustomOperation(1111, custom_op);
                let json = new_op_ctx.customOpToB64(1111);
                console.log("Custom operation Base64:\n", json);
            }
        },
        // copy paste me
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
finalizeButton.addEventListener("click", () => {
    // TODO: allow user to set the operation id (needs to happen at the beginning)
    let opId = parseInt(opIdInput.value);
    let op;
    try {
        op = op_builder.finalize();
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
    op_builder = OperationBuilder.create(opCtx, getNextOpId());
    updateState();

    console.log(`Finalized operation with id ${opId}`);
})