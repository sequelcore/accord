"""Send each snippet, with its questions, to each model through OpenRouter.

No system prompt and no tools; one new conversation per snippet.

Usage: OPENROUTER_API_KEY=... python run.py <model>[,<model>...] <repetitions>

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


def ask(model: str, snippet: str, repetition: int) -> dict:
    source = (HERE / "snippets" / f"{snippet}.txt").read_text(encoding="utf-8")
    code, questions = source.split("\n---\n")
    body = {
        "model": model,
        "max_tokens": 16000,
        "usage": {"include": True},
        "messages": [{"role": "user", "content": f"{PROMPT}\n```\n{code}\n```\n\n{questions}"}],
    }
    request = urllib.request.Request(
        "https://openrouter.ai/api/v1/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=300) as response:
        data = json.load(response)
    return {
        "model": data.get("model") or model,
        "snippet": snippet,
        "repetition": repetition,
        "reply": data["choices"][0]["message"].get("content") or "",
        "cost_usd": data.get("usage", {}).get("cost"),
    }


def main() -> None:
    models, repetitions = sys.argv[1].split(","), int(sys.argv[2])
    snippets = sorted(p.stem for p in (HERE / "snippets").glob("*.txt"))
    jobs = [(m, s, r) for m in models for s in snippets for r in range(1, repetitions + 1)]
    with concurrent.futures.ThreadPoolExecutor(8) as pool, open(
        HERE / "replies.jsonl", "a", encoding="utf-8", newline="\n"
    ) as out:
        for record in pool.map(lambda job: ask(*job), jobs):
            out.write(json.dumps(record, ensure_ascii=False) + "\n")
            print(record["model"], record["snippet"], record["repetition"], record["cost_usd"])


if __name__ == "__main__":
    main()
