"""Send each corpus file, with the prompt, to each model.

Through OpenRouter, with no system prompt and no tools; one new conversation
per file and repetition. A reply that comes back empty is asked once more.

Usage: OPENROUTER_API_KEY=... python run.py <repetitions>

Each reply is appended to replies.jsonl beside this file.
"""

import concurrent.futures
import json
import os
import pathlib
import sys
import urllib.request

HERE = pathlib.Path(__file__).parent
KEY = os.environ["OPENROUTER_API_KEY"]
PROMPT = (HERE / "prompt.txt").read_text(encoding="utf-8")
# The comprehension check's models (../comprehension-check/README.md).
MODELS = [
    "deepseek/deepseek-v4.1-flash",
    "qwen/qwen3.8-omni-flash",
    "anthropic/claude-haiku-5.5",
    "openai/gpt-6-luna",
    "mistralai/mistral-large-4-0",
    "z-ai/glm-5.3-flashx",
    "x-ai/grok-4.7",
]


def ask(model: str, file: str, repetition: int) -> dict:
    source = (HERE / "corpus" / f"{file}.accord").read_text(encoding="utf-8")
    body = {
        "model": model,
        "max_tokens": 16000,
        "usage": {"include": True},
        "messages": [{"role": "user", "content": f"{PROMPT}\n```accord\n{source}```\n"}],
    }
    for attempt in (1, 2):
        request = urllib.request.Request(
            "https://openrouter.ai/api/v1/chat/completions",
            data=json.dumps(body).encode(),
            headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"},
        )
        with urllib.request.urlopen(request, timeout=300) as response:
            data = json.load(response)
        reply = data["choices"][0]["message"].get("content") or ""
        if reply.strip() or attempt == 2:
            return {
                "model": data.get("model") or model,
                "file": file,
                "repetition": repetition,
                "attempt": attempt,
                "reply": reply,
                "cost_usd": data.get("usage", {}).get("cost"),
            }
    raise AssertionError("unreachable")


def main() -> None:
    repetitions = int(sys.argv[1])
    files = sorted(p.stem for p in (HERE / "corpus").glob("*.accord"))
    jobs = [(m, f, r) for m in MODELS for f in files for r in range(1, repetitions + 1)]
    with concurrent.futures.ThreadPoolExecutor(8) as pool, open(
        HERE / "replies.jsonl", "a", encoding="utf-8", newline="\n"
    ) as out:
        for record in pool.map(lambda job: ask(*job), jobs):
            out.write(json.dumps(record, ensure_ascii=False) + "\n")
            print(record["model"], record["file"], record["repetition"], record["cost_usd"])


if __name__ == "__main__":
    main()
