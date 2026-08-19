let meta = #{
    name: "prd-revise",
    description: "Freeze verifier leftovers in the text2mesh PRD pack then re-verify",
    when_to_use: "After prd workflow verify.md still has open issues",
    phases: [
        #{ title: "Revise", detail: "writer freezes seven leftovers" },
        #{ title: "Verify", detail: "independent skeptic, fail closed" },
    ],
};

let write_schema = #{
    "type": "object",
    "required": ["prd_path", "summary"],
    "properties": #{
        "prd_path": #{ "type": "string" },
        "summary": #{ "type": "string" },
    },
};

let review_schema = #{
    "type": "object",
    "required": ["path", "verdict", "open_count"],
    "properties": #{
        "path": #{ "type": "string" },
        "verdict": #{ "type": "string" },
        "open_count": #{ "type": "integer" },
    },
};

const ROOT = "/home/andre/Projects/Clean-Room/text2mesh";

phase("Revise");
log("Writer freezing verify.md Issues 1-7 in prd + design + CHARTER if needed");

let p = "";
p += "You are the PRD writer. Do NOT rewrite the pack. Edit in place. Bump Status to Draft v0.3.\n";
p += "HARD: no reference-project source. CHARTER D* win; freeze conflicts are bugs (D15).\n";
p += "Read ALL of:\n";
p += "  " + ROOT + "/docs/reviews/verify.md\n";
p += "  " + ROOT + "/docs/prd.md\n";
p += "  " + ROOT + "/docs/design.md\n";
p += "  " + ROOT + "/docs/CHARTER.md\n\n";
p += "Address EVERY Status: open issue in verify.md (Issues 1-7). Exact freezes:\n\n";
p += "ISSUE 1 — One wait default. Frozen: timeout_s min 30, default 1800, max 86400 on MCP, CLI, Route B max_wall_s.\n";
p += "Change PRD §9.3 tools table `text2mesh_wait` from default 600 to default 1800. Delete 600 as a wait default anywhere in prd/design.\n";
p += "Add to PR-01b / schema-drift: generated MCP tool default timeout_s equals CLI --timeout-s default (1800).\n\n";
p += "ISSUE 2 — One gate contract. PRD §7.7 MUST use canonical_view_id, not hero, for G0/G1/G4.\n";
p += "G2 FACE = exactly \"a face, two eyes, front of a head\". BACK = exactly \"the back of a head, no face\". DELETE the third string \"a face looking at camera\".\n";
p += "G0 text: clip_cos(canonical_view, identity_phrase) if identity_phrase non-empty else prompt.normalized. Not union/concat/max.\n";
p += "Amend FR-TXT-15 AND design invariant/ladder: after retries, a required view failing ANY of G1-G4 → do not call Image3dPlane. Keep the specific error_type of the gate that failed; wrap view.consistency on exhaust. Optional cameras may drop.\n";
p += "Align design §6 G0 line the same way (no identity ∪ normalized).\n";
p += "Sentence under PRD gate table: G1 vs canonical_view_id, plus G2 for faces. Preview cardinal4 has no hero.\n\n";
p += "ISSUE 3 — Freeze prompt strings so S5 goldens have one JSON.\n";
p += "Add prompt_suffix column to EVERY preset row in design §4.3. Frozen suffixes:\n";
p += "  front: \"front view, camera on +Z\"\n";
p += "  right: \"right side view\"\n";
p += "  back: \"back view, camera on -Z\"\n";
p += "  left: \"left side view\"\n";
p += "  hero: \"three-quarter view from the front-right\"\n";
p += "  top: \"top-down view\"\n";
p += "  qne: \"three-quarter view from the front-right, slightly higher\"\n";
p += "  qnw: \"three-quarter view from the front-left, slightly higher\"\n";
p += "Add class lock table in design §4.1 with exact strings:\n";
p += "  creature/character: lighting.prompt_lock=\"even overcast studio lighting, no hard shadows\"; background.prompt_lock=\"plain neutral gray background\"; style_lock.medium=\"photoreal\"; style_lock.prompt_lock=\"photoreal product photography, single subject\"\n";
p += "  product/prop: lighting=\"studio three-point lighting, soft key, fill, rim\"; background same gray; medium=photoreal; prompt_lock=\"photoreal product shot, single object, catalog\"\n";
p += "  architecture: lighting=\"even overcast daylight\"; background=\"plain neutral gray background\"; medium=photoreal; prompt_lock=\"architectural model, single building, no people\"\n";
p += "  vehicle: lighting=\"studio three-point lighting, soft key, fill, rim\"; background gray; medium=photoreal; prompt_lock=\"vehicle product shot, single vehicle, no riders\"\n";
p += "  unknown: same as product/prop\n";
p += "Janus negatives frozen exact JSON array for creature AND character:\n";
p += "[\"second face\",\"face on the back of the head\",\"two faces\",\"duplicate head\",\"extra limbs\",\"cropped limbs\",\"text\",\"watermark\",\"multiple subjects\",\"logo\"]\n";
p += "Other classes negatives frozen:\n";
p += "[\"text\",\"watermark\",\"logo\",\"multiple subjects\",\"cropped object\",\"hands holding the object\"]\n";
p += "PRD §7.4 example must use these exact strings. S5 goldens are a function of these tables.\n\n";
p += "ISSUE 4 — PRD §13.2: emit progress when the engine does; director parent-heartbeat while children run; watchdog may NOT treat missing lines as crash if pid is live. Point at design §8.1. Keep <=5 s as should cadence, not a kill. Delete \"Silence -> watchdog.\"\n\n";
p += "ISSUE 5 — Delete TEXT2MESH_CUSTOM_BASE_URL and TEXT2MESH_CUSTOM_KEY from PRD §12.3. Change cargo feature remote-http blurb to \"Meshy/Tripo/colony adapters (inert without keys).\" No remote.custom in v1.\n";
p += "If design §22 has no custom keys, leave it. Grep the three product docs for CUSTOM and custom adapter leftover.\n\n";
p += "ISSUE 6 — Put POST /v1/jobs/{id}/confirm in the FR-FAC-13 route block. Include ?kind= with kind including contract (as design §11).\n\n";
p += "ISSUE 7 — PR-10 checks in prompts.json AND identity.json. PR-13 checks in classify.json + species.txt (species file = closed list; inline tokens must match it). PR-11 may add evals/text2/scores/ fixtures. PR-21 uses the same files.\n\n";
p += "In verify.md: for each Issue 1-7 set Status: addressed and add Response: what you changed (file + section). If you wontfix, justify. Append a Revision Summary.\n";
p += "Update " + ROOT + "/docs/research/WRITEUP.md to v0.3.\n";
p += "Grep the three product docs for leftover 600 wait default, Silence -> watchdog, CUSTOM_BASE, a face looking at camera, identity_phrase ∪.\n";
p += "Return JSON {prd_path, summary} listing the seven fixes.";

let writer = agent(p, #{ label: "prd-v03-writer", capability_mode: "all", output_schema: write_schema });
if writer == () || !writer.success {
    pause("verification", "v0.3 writer failed");
}

phase("Verify");
log("Skeptic re-reads v0.3 against verify.md leftovers");

let s = "";
s += "Independent skeptic. Fail closed. Do not rewrite product docs.\n";
s += "Read " + ROOT + "/docs/reviews/verify.md then the REVISED docs/prd.md, design.md, CHARTER.md.\n";
s += "For Issues 1-7: is the Response true in the docs? If a residue remains, re-list as Status: open.\n";
s += "NEW issues only if this revision introduced a freeze conflict.\n";
s += "Write " + ROOT + "/docs/reviews/verify-v03.md with the same structured format.\n";
s += "open_count must be 0 only if Issues 1-3 are one contract in BOTH prd.md and design.md, and Issues 4-7 leftover sentences are gone.\n";
s += "Grep for: default 600, Silence → watchdog, TEXT2MESH_CUSTOM, a face looking at camera.\n";
s += "Return JSON {path, verdict, open_count}.";

let verify = agent(s, #{ label: "prd-v03-verify", capability_mode: "all", output_schema: review_schema });

let leftover = 99;
let verdict = "unknown";
if verify != () && verify.success {
    leftover = 0;
    if verify.output.open_count != () { leftover = verify.output.open_count; }
    if verify.output.verdict != () { verdict = verify.output.verdict; }
}

let report = "v0.3 leftover=" + leftover.to_string() + " verdict=" + verdict + "\n";
let path = write_scratch_file("prd-v03.md", report);
complete(#{ path: path, leftover: leftover, verdict: verdict, report: report });
