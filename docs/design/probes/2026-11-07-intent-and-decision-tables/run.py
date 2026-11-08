"""Send each variant to each model through OpenRouter, with no system prompt.

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


def ask(model: str, variant: str, repetition: int) -> dict:
    code = (HERE / "variants" / f"{variant}.txt").read_text(encoding="utf-8")
    body = {
        "model": model,
        "max_tokens": 16000,
        "usage": {"include": True},
        "messages": [{"role": "user", "content": f"{PROMPT}\n```\n{code}\n```"}],
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
        "variant": variant,
        "repetition": repetition,
        "reply": data["choices"][0]["message"].get("content") or "",
        "cost_usd": data.get("usage", {}).get("cost"),
    }


def main() -> None:
    models, repetitions = sys.argv[1].split(","), int(sys.argv[2])
    variants = sorted(p.stem for p in (HERE / "variants").glob("*.txt"))
    jobs = [(m, v, r) for m in models for v in variants for r in range(1, repetitions + 1)]
    with concurrent.futures.ThreadPoolExecutor(8) as pool, open(
        HERE / "replies.jsonl", "a", encoding="utf-8", newline="\n"
    ) as out:
        for record in pool.map(lambda job: ask(*job), jobs):
            out.write(json.dumps(record, ensure_ascii=False) + "\n")
            print(record["model"], record["variant"], record["repetition"], record["cost_usd"])


if __name__ == "__main__":
    main()
