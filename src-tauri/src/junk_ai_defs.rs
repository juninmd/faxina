//! Caches left behind by AI tooling: model runtimes, JIT kernels, agent CLIs and `npx`/MCP downloads.
//! Model stores the user pulled on purpose (Ollama, LM Studio) and chat transcripts are NOT listed.

use crate::junk_defs::{caches, Base, JunkDef, DEV};

pub fn definitions() -> Vec<JunkDef> {
    let mut v = vec![
        caches(
            "npx-cache",
            "npx e servidores MCP (pacotes baixados)",
            DEV,
            Base::Home,
            &[".npm/_npx", ".npm/_logs"],
        ),
        caches(
            "ai-weights",
            "Pesos de IA baixados (OCR, remoção de fundo, rostos, Keras)",
            DEV,
            Base::Home,
            &[
                ".u2net",
                ".EasyOCR/model",
                ".insightface/models",
                ".paddleocr",
                ".deepface/weights",
                ".keras/datasets",
                ".keras/models",
            ],
        ),
        caches(
            "gpu-jit",
            "Kernels de GPU compilados (Triton, CUDA, TorchInductor)",
            DEV,
            Base::Home,
            &[".triton/cache", ".nv/ComputeCache"],
        ),
        caches(
            "ai-temp",
            "Compilações temporárias de IA (TorchInductor, TF Hub)",
            DEV,
            Base::Temp,
            &["torchinductor_*", "tfhub_modules"],
        ),
        caches(
            "ai-agents",
            "Agentes de IA (Claude Code, Codex, Ollama: snapshots e logs)",
            DEV,
            Base::Home,
            &[
                ".claude/shell-snapshots",
                ".claude/statsig",
                ".codex/log",
                ".ollama/logs",
            ],
        ),
        caches(
            "conda-pkgs",
            "Conda/Mamba (pacotes baixados)",
            DEV,
            Base::Home,
            &[
                ".conda/pkgs",
                "miniconda3/pkgs",
                "anaconda3/pkgs",
                "miniforge3/pkgs",
                "mambaforge/pkgs",
                "micromamba/pkgs",
            ],
        ),
    ];
    v.extend(platform());
    v
}

/// `~/.cache` is offered whole on Linux, so these only exist as separate entries elsewhere.
#[cfg(any(windows, target_os = "macos"))]
fn home_cache() -> Vec<JunkDef> {
    vec![
        caches(
            "ai-datasets",
            "Datasets e áudio de IA (Hugging Face, Whisper)",
            DEV,
            Base::Home,
            &[
                ".cache/huggingface/datasets",
                ".cache/huggingface/xet",
                ".cache/huggingface/assets",
                ".cache/whisper",
            ],
        ),
        caches(
            "uv-home",
            "uv (cache de pacotes Python)",
            DEV,
            Base::Home,
            &[".cache/uv"],
        ),
    ]
}

#[cfg(windows)]
fn platform() -> Vec<JunkDef> {
    let mut v = home_cache();
    v.extend([
        caches(
            "npx-cache-local",
            "npx e servidores MCP (pacotes baixados)",
            DEV,
            Base::Local,
            &["npm-cache/_npx", "npm-cache/_logs"],
        ),
        caches(
            "claude-cli",
            "Claude Code (cache da CLI)",
            DEV,
            Base::Local,
            &["claude-cli-nodejs/Cache"],
        ),
        caches(
            "cuda-cache",
            "Cache de compilação CUDA (NVIDIA)",
            DEV,
            Base::Roaming,
            &["NVIDIA/ComputeCache"],
        ),
        caches(
            "jupyter-runtime",
            "Jupyter (arquivos de kernel em execução)",
            DEV,
            Base::Roaming,
            &["jupyter/runtime"],
        ),
    ]);
    v
}

#[cfg(target_os = "macos")]
fn platform() -> Vec<JunkDef> {
    let mut v = home_cache();
    v.push(caches(
        "jupyter-runtime",
        "Jupyter (arquivos de kernel em execução)",
        DEV,
        Base::Home,
        &["Library/Jupyter/runtime"],
    ));
    v
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform() -> Vec<JunkDef> {
    vec![caches(
        "jupyter-runtime",
        "Jupyter (arquivos de kernel em execução)",
        DEV,
        Base::Local,
        &["jupyter/runtime"],
    )]
}
