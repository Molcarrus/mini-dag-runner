// DAG Templates
const templates = {
    simple: {
        name: "Simple Linear Pipeline",
        tasks: [
            { id: "start", depends_on: [], command: "build" },
            { id: "middle", depends_on: ["start"], command: "test" },
            { id: "end", depends_on: ["middle"], command: "deploy" }
        ]
    },
    parallel: {
        name: "Parallel Tasks",
        tasks: [
            { id: "task_a", depends_on: [], command: "build" },
            { id: "task_b", depends_on: [], command: "test" },
            { id: "task_c", depends_on: [], command: "send_email" },
            { id: "final", depends_on: ["task_a", "task_b", "task_c"], command: "deploy" }
        ]
    },
    ci: {
        name: "CI/CD Pipeline",
        tasks: [
            { id: "checkout", depends_on: [], command: "build" },
            { id: "lint", depends_on: ["checkout"], command: "test" },
            { id: "unit_test", depends_on: ["checkout"], command: "test" },
            { id: "build", depends_on: ["lint", "unit_test"], command: "build" },
            { id: "integration_test", depends_on: ["build"], command: "test" },
            { id: "deploy_staging", depends_on: ["integration_test"], command: "deploy" },
            { id: "smoke_test", depends_on: ["deploy_staging"], command: "test" },
            { id: "deploy_prod", depends_on: ["smoke_test"], command: "deploy" },
            { id: "notify", depends_on: ["deploy_prod"], command: "send_email" }
        ]
    },
    failure: {
        name: "Pipeline with Failure",
        tasks: [
            { id: "task1", depends_on: [], command: "build" },
            { id: "task2_fail", depends_on: ["task1"], command: "fail" },
            { id: "task3_will_skip", depends_on: ["task2_fail"], command: "deploy" }
        ]
    }
};