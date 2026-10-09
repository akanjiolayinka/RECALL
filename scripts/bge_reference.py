"""Reference embeddings from the official BAAI/bge-small-en-v1.5 implementation.

Recall runs the model with its own Rust code (src-tauri/src/embeddings/bge.rs).
This script embeds a few texts with the model authors' recommended library,
sentence-transformers, so a Rust test can check that both give the same
vectors (`matches_the_reference_implementation`).

    pip install sentence-transformers
    python scripts/bge_reference.py <revision> models/bge-small-en-v1.5/reference.json

Needs internet access to Hugging Face. Development tool only; Recall itself
never runs Python.
"""

import json
import sys

from sentence_transformers import SentenceTransformer

# Must match QUERY_INSTRUCTION in src-tauri/src/embeddings/bge.rs.
QUERY_INSTRUCTION = "Represent this sentence for searching relevant passages: "

QUERIES = ["how much money does the garden project need", "headset purchase"]
PASSAGES = [
    "The estimated project budget is NGN 2,500,000 in total.",
    "Either party must give sixty days' written notice before moving out.",
    "Shopping list: tomatoes, peppers, garden gloves, watering can.",
]


def main(revision: str, output: str) -> None:
    model = SentenceTransformer("BAAI/bge-small-en-v1.5", revision=revision, device="cpu")
    texts = [(QUERY_INSTRUCTION + q, q, True) for q in QUERIES] + [(p, p, False) for p in PASSAGES]
    vectors = model.encode([t[0] for t in texts], normalize_embeddings=True)
    reference = [
        {"text": text, "query": is_query, "vector": vector.tolist()}
        for (_, text, is_query), vector in zip(texts, vectors)
    ]
    with open(output, "w", encoding="utf-8") as f:
        json.dump(reference, f)
    print(f"wrote {len(reference)} reference vectors to {output}")


if __name__ == "__main__":
    main(*sys.argv[1:3])
