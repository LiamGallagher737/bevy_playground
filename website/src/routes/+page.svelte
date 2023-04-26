<script lang="ts">
    import { onMount } from "svelte";
    import init, { bevy_playground_run_app } from "./game.js";

    const startingCode =
        'use bevy::prelude::*;\n\nfn main() {\n    App::new()\n        .add_plugins(DefaultPlugins)\n        .add_system(|| info!("Hello, World!"))\n        .run();\n}\n';

    let theme: "vs" | "vs-dark" = "vs-dark";

    onMount(async () => {
        const monaco = await import("monaco-editor");
        const editor = monaco.editor.create(
            document.getElementById("editor-container")!,
            {
                value: startingCode,
                language: "rust",
                theme,
            }
        );

        async function compile() {
            let code = editor.getValue();
            let req = await fetch("http://localhost:8080/compile", {
                method: "POST",
                body: JSON.stringify({
                    code,
                }),
                headers: {
                    "Content-Type": "application/json",
                },
            });
            await init(req);
            bevy_playground_run_app();
        }

        document.getElementById("run-btn")!.addEventListener("click", compile);
    });
</script>

<div id="editor-container" style="height: 500px;" />
<button id="run-btn">Run</button>
