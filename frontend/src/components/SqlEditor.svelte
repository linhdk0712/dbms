<script>
  /**
   * SqlEditor — Code editor component cho SQL/JSON input.
   *
   * Sử dụng CodeMirror 6 với:
   * - SQL syntax highlighting (mặc định)
   * - JSON syntax highlighting (cho MongoDB commands)
   * - Dark theme (one-dark) hoặc light tùy theo hệ thống
   * - Line numbers
   * - Bracket matching
   * - Auto-indent
   *
   * Props:
   * @prop {string} value - Nội dung editor (two-way binding)
   * @prop {string} language - "sql" | "json" (default: "sql")
   * @prop {boolean} readonly - Chỉ đọc (default: false)
   * @prop {string} placeholder - Placeholder text
   * @prop {number} minHeight - Chiều cao tối thiểu (px, default: 200)
   */

  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { EditorView, placeholder as cmPlaceholder, lineNumbers, keymap } from "@codemirror/view";
  import { EditorState } from "@codemirror/state";
  import { sql, PostgreSQL, MySQL } from "@codemirror/lang-sql";
  import { json } from "@codemirror/lang-json";
  import { oneDark } from "@codemirror/theme-one-dark";
  import { defaultKeymap, indentWithTab } from "@codemirror/commands";
  import {
    syntaxHighlighting,
    defaultHighlightStyle,
    bracketMatching,
    indentOnInput,
  } from "@codemirror/language";

  export let value = "";
  export let language = "sql";
  export let readonly = false;
  export let placeholder = "";
  export let minHeight = 200;

  const dispatch = createEventDispatcher();

  let editorContainer;
  let editorView;

  // Detect dark mode preference
  const prefersDark =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches;

  onMount(() => {
    const langExtension = language === "json" ? json() : sql({ dialect: PostgreSQL });

    const extensions = [
      lineNumbers(),
      bracketMatching(),
      indentOnInput(),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      langExtension,
      keymap.of([...defaultKeymap, indentWithTab]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          const newValue = update.state.doc.toString();
          if (newValue !== value) {
            value = newValue;
            dispatch("change", { value: newValue });
          }
        }
      }),
      EditorView.theme({
        "&": { minHeight: `${minHeight}px` },
        ".cm-scroller": { overflow: "auto" },
        ".cm-content": { fontFamily: "ui-monospace, monospace", fontSize: "14px" },
      }),
    ];

    // Theme
    if (prefersDark) {
      extensions.push(oneDark);
    }

    // Placeholder
    if (placeholder) {
      extensions.push(cmPlaceholder(placeholder));
    }

    // Readonly
    if (readonly) {
      extensions.push(EditorState.readOnly.of(true));
    }

    const state = EditorState.create({
      doc: value,
      extensions,
    });

    editorView = new EditorView({
      state,
      parent: editorContainer,
    });
  });

  onDestroy(() => {
    if (editorView) {
      editorView.destroy();
    }
  });

  // Sync external value changes into the editor
  $: if (editorView && value !== editorView.state.doc.toString()) {
    editorView.dispatch({
      changes: {
        from: 0,
        to: editorView.state.doc.length,
        insert: value,
      },
    });
  }
</script>

<div
  bind:this={editorContainer}
  class="border border-base-300 rounded-lg overflow-hidden"
  class:opacity-60={readonly}
></div>
