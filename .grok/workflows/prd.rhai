let meta = #{
    name: "prd",
    description: "Clean-room PRD loop for text2mesh: research, write, review, revise, verify",
    when_to_use: "Produce or revise the text2mesh PRD, CHARTER, and design contract",
    phases: [
        #{ title: "Research", detail: "four public-surface researchers" },
        #{ title: "Write", detail: "PRD + CHARTER + design contract" },
        #{ title: "Review", detail: "clean-room, completeness, feasibility" },
        #{ title: "Revise", detail: "writer addresses every open issue" },
        #{ title: "Verify", detail: "skeptic confirms zero open issues" },
    ],
};

let note_schema = #{
    "type": "object",
    "required": ["path", "summary"],
    "properties": #{
        "path": #{ "type": "string" },
        "summary": #{ "type": "string" },
    },
};

let write_schema = #{
    "type": "object",
    "required": ["prd_path", "charter_path", "design_path", "summary"],
    "properties": #{
        "prd_path": #{ "type": "string" },
        "charter_path": #{ "type": "string" },
        "design_path": #{ "type": "string" },
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
const BRIEF = "/home/andre/Projects/Clean-Room/text2mesh/docs/research/BRIEFING.md";

fn read_ok(r) {
    r != () && r.success
}

phase("Research");
log("Four researchers expanding the briefing from public sources only");

let rprompt_img = "";
rprompt_img += "You are a clean-room researcher. HARD RULE: do not open, clone, or quote source code from ";
rprompt_img += "AI_trellis2cpp, microsoft/TRELLIS, TRELLIS.2, Hunyuan3D, or TripoSR. Public README, papers, ";
rprompt_img += "model cards, and LICENSE only.\n\n";
rprompt_img += "Read " + BRIEF + " in full first.\n\n";
rprompt_img += "Task: write docs/research/image-to-3d.md under " + ROOT + ".\n";
rprompt_img += "Capture the FUNCTIONAL image-to-3d target: inputs, outputs (GLB/PBR), resolution tiers as ";
rprompt_img += "product quality names (not copied internals), backends (CPU+GPU vendors as capabilities), ";
rprompt_img += "job/server behaviour, license/GPL traps, and what we must independently design (stage ";
rprompt_img += "isolation, provenance, honesty). Mark [inferred] items. End with a short 'do not copy' list.\n";
rprompt_img += "Use web_search / web_fetch only against public docs if you need to confirm a fact. ";
rprompt_img += "Return JSON {path, summary}.";

let rprompt_txt = "";
rprompt_txt += "You are a clean-room researcher. HARD RULE: no reference-project source.\n";
rprompt_txt += "Read " + BRIEF + " in full first.\n\n";
rprompt_txt += "Task: write docs/research/text2-layer.md under " + ROOT + ".\n";
rprompt_txt += "Evaluate text-to-3d candidates (TRELLIS-text, T2I-then-image-3d, Hunyuan, Tripo/Meshy APIs, ";
rprompt_txt += "Cadre analytic). Recommend a concrete invention for the text2 layer: typed View Contract + ";
rprompt_txt += "Lattice Router (or a strictly better original design). Specify: contract fields, camera ";
rprompt_txt += "ring, consistency gate, retry budget, sibling compose (Imaginarium/Cadre), and why this is ";
rprompt_txt += "not a placeholder. License-vet Hunyuan. End with a recommended default + two alternatives.\n";
rprompt_txt += "Return JSON {path, summary}.";

let rprompt_cmp = "";
rprompt_cmp += "You are a clean-room researcher. HARD RULE: no reference-project source.\n";
rprompt_cmp += "Read " + BRIEF + " in full first.\n\n";
rprompt_cmp += "Task: write docs/research/compute-plane.md under " + ROOT + ".\n";
rprompt_cmp += "Design the dual-path compute requirement in enough detail for a PRD: ComputePlane trait, ";
rprompt_cmp += "Local vs Remote vs Auto planner, job state machine (never stuck pending), spend gates, ";
rprompt_cmp += "system-check honesty, Nano-first (feature-gate heavy engines), sidecar vs in-process, ";
rprompt_cmp += "inference runtime options (candle/burn/ggml-FFI/onnx) with a recommendation, VRAM/disk ";
rprompt_cmp += "ballparks from public cards, and mockability for CI. Return JSON {path, summary}.";

let rprompt_hs = "";
rprompt_hs += "You are a clean-room researcher. HARD RULE: no reference-project source.\n";
rprompt_hs += "Read " + BRIEF + " and these house files if present:\n";
rprompt_hs += "  /home/andre/Projects/Launchpad-RS/docs/house-doctrine.md\n";
rprompt_hs += "  /home/andre/Projects/Launchpad-RS/docs/stack.md\n";
rprompt_hs += "  /home/andre/Projects/OmniOcular-RS/docs/CHARTER.md\n";
rprompt_hs += "  /home/andre/Projects/Cadre-RS/docs/CHARTER.md\n";
rprompt_hs += "  /home/andre/Projects/Scriptum-RS/DocSmith-PRD-Iterated-Opus-123/docsmith-prd.md (section 0 only)\n\n";
rprompt_hs += "Task: write docs/research/house-and-siblings.md under " + ROOT + ".\n";
rprompt_hs += "Extract: four-face shape, MCP pin, Nano-first, spend, honesty, sibling compose boundaries ";
rprompt_hs += "(Cadre vs Imaginarium vs OmniOcular vs this product), DocSmith PRD section list, suggested ";
rprompt_hs += "crate map, suggested HTTP port (avoid 8791/8795/7411/7320/8788), Cerebro agent id. ";
rprompt_hs += "Return JSON {path, summary}.";

let research = parallel([
    #{ prompt: rprompt_img, label: "research:image-to-3d", capability_mode: "all", output_schema: note_schema },
    #{ prompt: rprompt_txt, label: "research:text2", capability_mode: "all", output_schema: note_schema },
    #{ prompt: rprompt_cmp, label: "research:compute", capability_mode: "all", output_schema: note_schema },
    #{ prompt: rprompt_hs, label: "research:house", capability_mode: "all", output_schema: note_schema },
]);

let i = 0;
while i < research.len() {
    if !read_ok(research[i]) {
        log("research slot " + i.to_string() + " failed or empty — writer must still use BRIEFING.md");
    }
    i += 1;
}

phase("Write");
log("PRD writer drafting charter, PRD, and design contract");

let wprompt = "";
wprompt += "You are an experienced systems architect writing a clean-room PRD for a greenfield Rust project.\n";
wprompt += "HARD RULES:\n";
wprompt += "- Do not open or copy source from AI_trellis2cpp / TRELLIS / Hunyuan / TripoSR.\n";
wprompt += "- Independent types, wire formats, file formats, crate names, and stage names.\n";
wprompt += "- House doctrine: contract first, honesty, spend gated, Nano-first, standalone-first.\n";
wprompt += "- You MUST write three files to disk (create directories if needed).\n\n";
wprompt += "Read in this order, using read_file:\n";
wprompt += "1) " + BRIEF + "\n";
wprompt += "2) " + ROOT + "/docs/research/image-to-3d.md\n";
wprompt += "3) " + ROOT + "/docs/research/text2-layer.md\n";
wprompt += "4) " + ROOT + "/docs/research/compute-plane.md\n";
wprompt += "5) " + ROOT + "/docs/research/house-and-siblings.md\n";
wprompt += "If a research file is missing, proceed from BRIEFING.md and say so in the PRD provenance.\n\n";
wprompt += "WRITE THESE FILES:\n\n";
wprompt += "A) " + ROOT + "/docs/prd.md — Clean-room PRD, DocSmith-shaped, with ALL of:\n";
wprompt += "   Status Draft v0.1, date 2026-08-19, working name text2mesh / recommended garden name.\n";
wprompt += "   0 Clean-Room Methodology & Provenance\n";
wprompt += "   1 Summary (one paragraph a senior engineer can act on)\n";
wprompt += "   2 Background & Problem\n";
wprompt += "   3 Goals\n";
wprompt += "   4 Non-Goals (permanent vs deferred)\n";
wprompt += "   5 Personas (human maker, agent harness, colony node, offline appliance)\n";
wprompt += "   6 Functional requirements — image-to-mesh (numbered FR-IMG-*)\n";
wprompt += "   7 Functional requirements — text2 layer invention (FR-TXT-*) — MUST be a real invention\n";
wprompt += "      with typed View Contract fields, Lattice Router, consistency loop, measurable gate\n";
wprompt += "   8 Functional requirements — compute plane dual path (FR-CMP-*)\n";
wprompt += "   9 Functional requirements — faces MCP/CLI/HTTP/UI (FR-FAC-*)\n";
wprompt += "   10 Non-functional requirements (NFR-*)\n";
wprompt += "   11 Proposed Architecture (ORIGINAL — mermaid diagrams, crate map, job state machine)\n";
wprompt += "   12 Security, licenses, spend, secrets\n";
wprompt += "   13 Observability (manifest, timings, system-check)\n";
wprompt += "   14 Milestones S0–Sn with checkable done-whens\n";
wprompt += "   15 Success metrics\n";
wprompt += "   16 Alternatives considered (at least 3, with why not)\n";
wprompt += "   17 Key Decisions (numbered, with rationale) — MANDATORY\n";
wprompt += "   18 Open Questions (do not silently resolve house-briefing OQ-1..7; pick a recommendation\n";
wprompt += "      labelled Recommended and keep them as OQ if André must decide)\n";
wprompt += "   19 PR Plan — ordered, independently mergeable PRs: title, files, deps, description — MANDATORY\n";
wprompt += "   20 Glossary\n";
wprompt += "   Appendix A — illustrative original tool/HTTP schemas (not copied)\n";
wprompt += "   Appendix B — sources reviewed (URLs only)\n";
wprompt += "   Be specific: types, JSON examples, error codes, env vars, ports, feature flags.\n";
wprompt += "   Quantify: latency bands, disk (~14GB class weights), VRAM bands from public cards.\n";
wprompt += "   The text2 layer is the load-bearing invention — do not leave it as 'call an API'.\n\n";
wprompt += "B) " + ROOT + "/docs/CHARTER.md — Launchpad CHARTER: what it is / is not, D1–Dn dated 2026-08-19,\n";
wprompt += "   phases table, deliberately out, open questions, amendments. Binding decisions only.\n\n";
wprompt += "C) " + ROOT + "/docs/design.md — the contract: job schema, state machine, MCP tools, HTTP routes,\n";
wprompt += "   env vars, honest degrades, invariants. Code will follow this document.\n\n";
wprompt += "Also write a one-page " + ROOT + "/docs/research/WRITEUP.md summarizing what you produced.\n";
wprompt += "Return JSON {prd_path, charter_path, design_path, summary}.";

let writer = agent(wprompt, #{
    label: "prd-writer",
    capability_mode: "all",
    output_schema: write_schema,
});

if !read_ok(writer) {
    pause("verification", "PRD writer failed — inspect /workflows and resume after fixing.");
}

phase("Review");
log("Three adversarial reviewers");

let rev_cr = "";
rev_cr += "You are a senior staff engineer reviewing a CLEAN-ROOM PRD. Do not rewrite the docs.\n";
rev_cr += "Read " + BRIEF + " then " + ROOT + "/docs/prd.md, CHARTER.md, design.md in full.\n";
rev_cr += "Hunt: any copied internals (t2_* ABI, .t2mesh, trellis2_ function names, ggml graph details),\n";
rev_cr += "any instruction to implementers to read reference source, any Hunyuan-as-default, any fake\n";
rev_cr += "success paths, any missing provenance section. Write structured issues to\n";
rev_cr += ROOT + "/docs/reviews/clean-room.md using:\n";
rev_cr += "## Issue N: title\n- Severity: critical|major|minor|nit\n- Section:\n- Description:\n- Suggestion:\n- Status: open\n";
rev_cr += "End with a Summary verdict. Return JSON {path, verdict, open_count}.";

let rev_cp = "";
rev_cp += "You are a senior staff engineer reviewing completeness. Do not rewrite the docs.\n";
rev_cp += "Read " + ROOT + "/docs/prd.md, CHARTER.md, design.md.\n";
rev_cp += "Checklist: all mandatory PRD sections; Key Decisions; PR Plan ordered and mergeable;\n";
rev_cp += "text2 invention specific enough to implement (typed fields, not slogans); dual compute\n";
rev_cp += "with Auto planner; MCP/CLI/HTTP parity; job state machine never-pending; Nano feature\n";
rev_cp += "gates; sibling compose; measurable success metrics; alternatives. Write issues to\n";
rev_cp += ROOT + "/docs/reviews/completeness.md in the same structured format, Status: open.\n";
rev_cp += "Return JSON {path, verdict, open_count}.";

let rev_fs = "";
rev_fs += "You are a senior staff engineer reviewing feasibility and house-fit. Do not rewrite.\n";
rev_fs += "Read " + ROOT + "/docs/prd.md, CHARTER.md, design.md and Launchpad stack.md if needed.\n";
rev_fs += "Hunt: v1 that secretly requires a from-scratch TRELLIS.2 reimplementation before any demo;\n";
rev_fs += "unrealistic VRAM claims; stacked-PR traps; GPL pulled in by default; crate map vs four-face;\n";
rev_fs += "timeouts <30s; ApexOS ownership; missing mock engine for CI; PR Plan that is not sliced.\n";
rev_fs += "Write issues to " + ROOT + "/docs/reviews/feasibility.md structured, Status: open.\n";
rev_fs += "Return JSON {path, verdict, open_count}.";

let reviews = parallel([
    #{ prompt: rev_cr, label: "review:clean-room", capability_mode: "all", output_schema: review_schema },
    #{ prompt: rev_cp, label: "review:completeness", capability_mode: "all", output_schema: review_schema },
    #{ prompt: rev_fs, label: "review:feasibility", capability_mode: "all", output_schema: review_schema },
]);

let open_total = 0;
let j = 0;
while j < reviews.len() {
    if read_ok(reviews[j]) && reviews[j].output.open_count != () {
        open_total += reviews[j].output.open_count;
    } else {
        open_total += 1;
        log("review slot " + j.to_string() + " unusable — treat as at least one open issue");
    }
    j += 1;
}
log("open issues reported: " + open_total.to_string());

phase("Revise");
log("Writer addressing every open issue");

let vprompt = "";
vprompt += "The reviewers filed issues. Address ALL Status: open items, including nits.\n";
vprompt += "Read:\n";
vprompt += "  " + ROOT + "/docs/reviews/clean-room.md\n";
vprompt += "  " + ROOT + "/docs/reviews/completeness.md\n";
vprompt += "  " + ROOT + "/docs/reviews/feasibility.md\n";
vprompt += "  and the current docs/prd.md, CHARTER.md, design.md under " + ROOT + "\n\n";
vprompt += "Revise the three product docs in place. For each issue, in the review file:\n";
vprompt += "  change Status: open -> Status: addressed (or wontfix / needs-user-input)\n";
vprompt += "  add a Response: field explaining the change.\n";
vprompt += "Push back with wontfix only when the suggestion would make the design worse; justify.\n";
vprompt += "needs-user-input only for true product forks André must pick (keep the Recommended line).\n";
vprompt += "Bump PRD status to Draft v0.2. Append a Revision Summary to each review file.\n";
vprompt += "Update " + ROOT + "/docs/research/WRITEUP.md.\n";
vprompt += "Return JSON {prd_path, charter_path, design_path, summary}.";

let writer_id = ();
if writer != () && writer.agent_id != () {
    writer_id = writer.agent_id;
}

let revise_opts = #{
    label: "prd-writer-revise",
    capability_mode: "all",
    output_schema: write_schema,
};
if writer_id != () {
    revise_opts.resume_from = writer_id;
}

let revised = agent(vprompt, revise_opts);
if !read_ok(revised) {
    pause("verification", "Revise writer failed — inspect review files and resume.");
}

phase("Verify");
log("Independent skeptic — fail closed");

let sprompt = "";
sprompt += "You are an independent skeptic. Re-read the REVISED docs and the three review files.\n";
sprompt += "Paths under " + ROOT + ": docs/prd.md, docs/CHARTER.md, docs/design.md, docs/reviews/*.md\n";
sprompt += "Confirm every former open issue is addressed or wontfix with a real Response.\n";
sprompt += "List NEW issues if the revision introduced problems. Do not rubber-stamp.\n";
sprompt += "Write " + ROOT + "/docs/reviews/verify.md with structured issues (Status: open) or an explicit\n";
sprompt += "statement that open_count is 0 and the PRD is ready for André.\n";
sprompt += "Fail closed: if a review file is missing or a Response is empty, that is an open issue.\n";
sprompt += "Return JSON {path, verdict, open_count}.";

let verify = agent(sprompt, #{
    label: "prd-verify",
    capability_mode: "all",
    output_schema: review_schema,
});

let leftover = 0;
let verdict = "unknown";
if read_ok(verify) {
    if verify.output.open_count != () { leftover = verify.output.open_count; }
    if verify.output.verdict != () { verdict = verify.output.verdict; }
} else {
    leftover = 99;
}

let report = "";
report += "# text2mesh PRD workflow result\n\n";
report += "verdict: " + verdict + "\n";
report += "verify_open_count: " + leftover.to_string() + "\n";
report += "prd: " + ROOT + "/docs/prd.md\n";
report += "charter: " + ROOT + "/docs/CHARTER.md\n";
report += "design: " + ROOT + "/docs/design.md\n";
if leftover > 0 {
    report += "\nNOT CLEAN. Remaining issues in docs/reviews/verify.md — do not treat as done.\n";
} else {
    report += "\nVerifier reports 0 open issues. Present Key Decisions, PR Plan, and Open Questions to André.\n";
}

let report_path = write_scratch_file("prd-result.md", report);
complete(#{
    path: report_path,
    report: report,
    leftover: leftover,
    verdict: verdict,
});
