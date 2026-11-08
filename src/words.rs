//! 語の記録 -- 登録した語 (何を指すか)、廃止した語や不自然な日本語 (代わりに何と書くか)、
//! 標準語と認めた略語 (展開を求めない) を 1 語 1 項で持つデータベース。この表と検査規則 (src/rules.rs、
//! 条件なしで避ける書き方はそちらに置く) を合わせたものが、sumi が語を登録 ・廃止した
//! 唯一の記録である。
//!
//! 編集: 登録 ・廃止 ・判定は操作者の裁定に続けて足す。形はこのファイルの単体テスト
//! (`cargo test` の `words::tests`) が検査する。置換先は文脈に合わせて選び、単語だけを機械的に
//! 置き換えて意味が不明確になるなら文全体を組み直す。識別子 ・設定キー ・状態値 ・コマンド ・パス ・URL は
//! 原語のまま置く。
//!
//! 登録簿を 1 か所に置く理由: 「追従」「発話」「段」「掃引」「ゲージ」「計器」「証人」のように、設計文書と
//! ソースの間で 2 つ以上の意味を持つ語があり、各文書の頭で決めると食い違う。廃止した語は追跡ファイル
//! 全体を grep で探せるが、keep に挙げた別の語をヒットから除く。
//!
//! 複数行の文字列は各行の末尾に改行を持つ (最後の行も含む)。

/// 登録した語。`word` が日本語の書き方、`ident` がコードや英語の識別子、`meaning` が何を指すか、
/// `not` が指さないもの、`verb` が動詞形、`note` が背景。
pub struct Register {
    pub word: &'static str,
    pub ident: &'static [&'static str],
    pub meaning: &'static str,
    pub not: &'static [&'static str],
    pub verb: &'static [&'static str],
    pub note: Option<&'static str>,
}

/// 避ける書き方と代わりに書く表現。`when` が条件、`keep` は同じ字を含む別の語として検出から除く
/// 書き方、`note` は背景。
pub struct Rewrite {
    pub avoid: &'static [&'static str],
    pub write: &'static [&'static str],
    pub when: Option<&'static str>,
    pub keep: &'static [&'static str],
    pub note: Option<&'static str>,
}

/// 一般語がその一般的な意味で使われていて、登録語と衝突しないと判定した例。
pub struct Ordinary {
    pub word: &'static str,
    pub note: &'static str,
}

/// 略語のうち、操作者が「読み手が既に知る標準の語」と認めたもの。
pub struct Abbreviation {
    pub word: &'static str,
    pub note: &'static str,
}

// ---------------------------------------------------------------------------------------------
// 登録した語
// ---------------------------------------------------------------------------------------------

pub static REGISTER: &[Register] = &[
    Register {
        word: "エージェント",
        ident: &["agent"],
        meaning: concat!(
            "タスクがバインドされる持続的な主体。目的、規範、ペルソナ、記憶によって個体性が規定される静的な\n",
            "定義 (DESIGN.md Section A1)。存在するエージェントを決めるのは文書でなくディレクトリである。木は初めて必要に\n",
            "なったときに agents/<id> として作り、ハーネスはそのディレクトリにあるものを列挙し、supervisor は\n",
            "見つけたものをバインドし、数え、解放する。\n",
        ),
        not: &[
            "タスク (作業そのもの)",
            "agent-instance (エージェント定義と llm-instance の組)",
        ],
        verb: &[],
        note: None,
    },
    Register {
        word: "アシスタント",
        ident: &["assistant"],
        meaning: concat!(
            "操作者と直接話すエージェント。汎用 policy/ では agent を「言語モデルが駆動する主体すべて」の広義で\n",
            "使い、assistant はそのうち操作者のチャネルを持つもの (must/consult-policy-before-acting の定義)。\n",
        ),
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "agent-instance",
        ident: &[],
        meaning: "エージェント定義と llm-instance の組み合わせ。実際に動作する主体を指す。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "llm-instance",
        ident: &["llm-instance", "LLM インスタンス"],
        meaning: "モデルウェイト、GPU、バックエンド、各種設定の組み合わせで特定される LLM リクエスト先。\n",
        not: &["instance 単独 (systemd インスタンスを指す)"],
        verb: &[],
        note: None,
    },
    Register {
        word: "スロット",
        ident: &["slot"],
        meaning: "LLM サーバーが提供する同時実行の単位 (DESIGN.md Section A9.8)。",
        not: &[],
        verb: &[],
        note: Some(concat!(
            "どの層を指すかを書く。ハーネスがサーバーに対して持つ管理上の割り当てか、バックエンドのプロセス自体が\n",
            "1 台を分ける処理容量か。\n",
        )),
    },
    Register {
        word: "トップレベル",
        ident: &["top-level"],
        meaning: "人間または claude-remote が保持する最上位の決定権限 (DESIGN.md と policy/README.md の語)。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "バックエンド",
        ident: &[],
        meaning: "ollama、llama.cpp、vLLM などの推論ソフトウェア。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "ハーネス",
        ident: &[],
        meaning: "LLM の入出力をすべて中継し、読み書きするプログラム。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "ツール",
        ident: &[],
        meaning: "LLM が呼び出して外部とのやり取りや状態参照を行うハーネスの機能。副作用の有無を問わない。",
        not: &["ゲート", "検証器"],
        verb: &[],
        note: None,
    },
    Register {
        word: "プロンプト",
        ident: &["インストラクション"],
        meaning: "ハーネスが LLM への入力に付加するデータ。広義には LLM への入力全般。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "コネクタ",
        ident: &["connector"],
        meaning: concat!(
            "投入元を認証し、署名した操作をサービスへ渡す常駐プログラム。\n",
            "投入元と、操作を検査する機能の間に置く。\n",
        ),
        not: &[
            "ゲートの側で投入の範囲を検分する段階",
            "投入元そのもの (ホストの調整役のセッション)",
            "自分の uid の鍵でそのまま署名する CLI 形の産出側",
        ],
        verb: &[],
        note: Some("指さないものはいずれも、投入元を認証する段階を持つコネクタと異なる。"),
    },
    Register {
        word: "ゲート",
        ident: &[],
        meaning: "要求やラウンドが通過すべき検証 ・検査機能。投入された操作が通るゲート、ラウンドを閉じるための検査、host check-*。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "テーブル",
        ident: &[],
        meaning: "ハーネスがレコード群として管理するファイル。state/subtasks.json、nonce の表、ratchet の表、steers.json。",
        not: &["Markdown の表 (「表」と書く)"],
        verb: &[],
        note: None,
    },
    Register {
        word: "UI",
        ident: &["チャット UI", "操作者 UI"],
        meaning: "操作者が情報を確認し操作を行う画面や表示。ダッシュボードとそのタブ、Chat のセクション、ctl chat の表示。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "ノード",
        ident: &["node"],
        meaning: "uniqnode グラフにおける頂点。node は uniqnode の宛先を指す。",
        not: &[],
        verb: &[],
        note: Some(
            "以前の書き方は「節点」 (2026-10-26T13:54Z 操作者の用語裁定で「ノード」へ替えた)。",
        ),
    },
    Register {
        word: "タスク",
        ident: &["task"],
        meaning: concat!(
            "ひとまとまりの作業。tasks/<name> と、その task.toml、work/、state/。アプリケーション層が数える単位は\n",
            "タスク、ラウンド、ステップの順に入れ子で、その下でエージェントを割り当てる機構がバインドと\n",
            "コンテキストスパンである。\n",
        ),
        not: &[
            "操作者の 1 回の発話への 1 つの応答",
            "タスクにバインドされる主体 (エージェント)",
        ],
        verb: &[],
        note: None,
    },
    Register {
        word: "ラウンド",
        ident: &["round"],
        meaning: concat!(
            "操作者の発話から、終了を示す break または finish が受理されるまでの区間。state/history.jsonl の 1 行、\n",
            "開いている間は state/round.json。\n",
        ),
        not: &["LLM の 1 回の生成 (「生成」と書く)"],
        verb: &[],
        note: None,
    },
    Register {
        word: "ステップ",
        ident: &["step"],
        meaning: concat!(
            "LLM への 1 回の呼び出しとその出力、およびそれに続くツール呼び出しと観測結果。state/round_steps.jsonl の\n",
            "1 行、ev=step の 1 件。\n",
        ),
        not: &[
            "todo の steps 配列に含まれる計画上の項目 (同じ書き方の別の対象。計画の項で、LLM 呼び出しとは別の対象)",
        ],
        verb: &[],
        note: Some(
            "日本語でこの単位を「手」と書くのは廃止 (rewrite の項を参照)。手順の一般的な「ステップ」は ordinary の項を参照。",
        ),
    },
    Register {
        word: "生成",
        ident: &["generation"],
        meaning: "LLM による 1 回の生成処理と、その中断処理。ログ事象 gen、gen_aborted、gen_salvaged が運ぶ語。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "バインド",
        ident: &["bind"],
        meaning: concat!(
            "エージェントを 1 つのタスクへ割り当てる操作と、それを解除する操作 (ctl bind、ctl release、およびゲート経由で\n",
            "投入される同じ 2 操作)。\n",
        ),
        not: &[
            "ファイルシステムの bind mount (常に bind mount、bind path のように複合語で書く)",
            "バインドが続く区間そのもの (コンテキストスパン)",
        ],
        verb: &["バインドする", "バインドされる", "バインドできる"],
        note: Some(concat!(
            "日本語の動詞は登録語の動詞形「バインドする」「バインドされる」であり、「束縛する」「割り当てる」は\n",
            "使わない (2026-09-19 操作者裁定。should/japanese-prose-style の「動詞は日本語で」はこの登録に譲る)。\n",
            "既存の「束縛する」は、その文書を次に触るときに「バインドする」へ寄せる。\n",
        )),
    },
    Register {
        word: "Writer",
        ident: &["BindKind::Normal", "writer"],
        meaning: "タスクの work/ に書き込む通常のバインド (の実行)。1 つのタスクに同時に 1 つだけ居る。\n",
        not: &[
            "ファイルや記録を書くコードの意味の「書き手」(その意味ではそのまま使う)",
            "work/ を書かないバインド (Observer)",
        ],
        verb: &[],
        note: Some(concat!(
            "Writer と Observer の組で使う (2026-10-13T10:56Z 操作者の用語裁定)。既存の「書き手」のうち\n",
            "この意味のものは、その文書を次に触るときに「Writer」へ寄せる。\n",
        )),
    },
    Register {
        word: "Observer",
        ident: &["BindKind::OBSERVATIONS", "observational"],
        meaning: concat!(
            "タスクの work/ を書かず、自分の記録だけを書く観測専用のバインド (証人、steer 審査、ヘルプデスク)。\n",
            "ReadOnly のファイル資源に書き込まない (BindKind::writes_work が偽のバインド)。\n",
        ),
        not: &["計測層の観測面", "測定や観察そのもの (観測期間、観測結果)"],
        verb: &[],
        note: Some(concat!(
            "Writer と Observer の組で使う。裁定の文は観測を行う主体を「観測タスク」と呼ぶので、「観測タスクが\n",
            "誤って ReadOnly のファイル資源に書き込まないこと」のように、禁じる書き込みの主体と対象を書く。\n",
            "「観測が誤って書けない境界」の形を避ける (2026-10-13T10:55Z と 10:56Z 操作者の用語裁定)。\n",
        )),
    },
    Register {
        word: "コンテキストスパン",
        ident: &["context_span"],
        meaning: concat!(
            "1 つのバインドが続く区間で、モデルの記憶と連続性がその上を走る。ログの 1 セグメント logs/<src>/<sid>/\n",
            "であり、プロンプトの射影、空回りの監視、繰り返しの検出、rollup が読むもの。\n",
        ),
        not: &["文脈長 (ctx、num_ctx)"],
        verb: &[],
        note: None,
    },
    Register {
        word: "問答",
        ident: &["exchange"],
        meaning: concat!(
            "操作者との会話以外の対話における、1 つの質問とそれに対する回答。helpdesk の質問 (HELPDESK_EXCHANGES_MAX)、\n",
            "審査中の steer (STEER_REVIEW_EXCHANGES_MAX)、witness の質問 (WITNESS_EXCHANGES_MAX)、transcript の\n",
            "## exchange 見出し 1 つ。\n",
        ),
        not: &["ラウンド (1 語で両方を数えると、別の 2 つの対話を同じ単位で数えてしまう)"],
        verb: &[],
        note: None,
    },
    Register {
        word: "文脈長",
        ident: &[],
        meaning: "ctx や num_ctx によって定められるトークン長 (コンテキスト長)。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "期間",
        ident: &[],
        meaning: "時間的な範囲。「測定期間」「観測期間」のように書く。",
        not: &[],
        verb: &[],
        note: Some(
            "「期間を設ける」「期間が終了する」と組み、「開く」「閉じる」と組む形を避ける。",
        ),
    },
    Register {
        word: "範囲",
        ident: &[],
        meaning: "対象の広がり。「可視範囲」「記憶範囲」のように何の範囲かを書く。",
        not: &[],
        verb: &[],
        note: Some("件数の上限を指す場合は「件数の上限」と書く。"),
    },
    Register {
        word: "ウィンドウ",
        ident: &[],
        meaning: "OS やブラウザの表示ウィンドウ。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "停止の時間帯",
        ident: &[],
        meaning: "サービスを停止して作業を通す時間帯。",
        not: &[],
        verb: &[],
        note: Some(concat!(
            "何を止め、止めている間にどの機能が使えなくなるかを近くの文で添える (「いま配っているモデルを止めてよいか」\n",
            "「止めている間に使うコマンド」)。名詞だけを置いて動詞を読み手に補わせない。名詞「停止の時間帯」を\n",
            "許すのは 2026-09-19 の操作者裁定で、それまでの「名詞に畳まず文で書く」を置き換える。\n",
        )),
    },
    Register {
        word: "群",
        ident: &[],
        meaning: "A/B テスト等で比較対象を分類したグループ。「A 群」「B 群」「両群」「焦点の群」。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "タスクグループ",
        ident: &[],
        meaning: "親タスクとその子孫タスクからなる集合。「タスクグループのルート」「タスクグループの上限」「タスクグループ外の読み取り」。",
        not: &[],
        verb: &[],
        note: Some(concat!(
            "同じ文で Unix のグループを指すときは「Unix グループ」と書いて分ける (「uid の粒度はタスク、Unix グループの\n",
            "粒度はタスクグループである」)。名前つきの fam_<root> グループと共有 agents グループはそのまま名前で呼ぶ。\n",
            "識別子 family_root と Unix グループ名 fam_<root> は残る。\n",
        )),
    },
    Register {
        word: "Unix グループ",
        ident: &[],
        meaning: "gid で識別される OS のグループ。タスクグループと区別して書き、固有のグループは agents などの名前で呼ぶ。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "エッジ",
        ident: &[],
        meaning: "状態遷移図やグラフの接続。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "事前実証実験",
        ident: &[],
        meaning: "PoC やプロトタイプ検証。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "ループ",
        ident: &["サイクル"],
        meaning: "繰り返し処理の 1 回分 (supervisor の主ループ、agentd のステップループ)。「次のループ」「同じループの中で」「サイクル A」。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "指示対象",
        ident: &[],
        meaning: "英語の referent。語が何を指すか。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "デプロイ",
        ident: &[],
        meaning: "deploy。ビルドした版を稼働系へ反映すること。",
        not: &[],
        verb: &[],
        note: None,
    },
    Register {
        word: "ベースライン",
        ident: &[],
        meaning: "baseline。比較の基準となる測定や構成。",
        not: &[],
        verb: &[],
        note: None,
    },
];

// ---------------------------------------------------------------------------------------------
// 避ける書き方と、代わりに書く表現
// ---------------------------------------------------------------------------------------------

pub static REWRITE: &[Rewrite] = &[
    Rewrite {
        avoid: &["セッション", "session"],
        write: &["バインド", "コンテキストスパン"],
        when: None,
        keep: &[],
        note: Some(concat!(
            "この系の対象を指す「セッション」は、操作 (割り当てと解除) の「バインド」か、1 つのバインドが続く\n",
            "区間の「コンテキストスパン」のどちらかを指しており、木はその 2 つを既に分けているので、両方を\n",
            "1 語で呼ぶと、どちらを指すかが失われる。日本語でも英語でも、識別子とモジュール名まで廃止。Claude Code や\n",
            "Codex など外部エージェントの作業単位を指す「セッション」(CLAUDE.md と\n",
            "should/editing-sessions-own-separate-worktrees が言う単位) は別の系の対象で、そのまま使う\n",
            "(2026-09-19 操作者裁定)。\n",
        )),
    },
    Rewrite {
        avoid: &["手"],
        write: &["ステップ"],
        when: Some("LLM 呼び出しの単位を指す場合"),
        keep: &[
            "着手",
            "書き手",
            "読み手",
            "呼び手",
            "答え手",
            "手順",
            "次の一手",
            "操作者の手",
        ],
        note: None,
    },
    Rewrite {
        avoid: &["ラウンド"],
        write: &["生成"],
        when: Some("LLM の 1 回の生成を指す場合"),
        keep: &[],
        note: Some(
            "操作者の発話から break / finish の受理までの区間を指す「ラウンド」は登録語で、そのまま使う。",
        ),
    },
    Rewrite {
        avoid: &["面"],
        write: &["UI", "チャット UI", "操作者 UI"],
        when: Some(
            "操作者が読み、操作する画面や表示 (ダッシュボードとそのタブ、Chat のセクション、ctl chat の表示) を指す場合",
        ),
        keep: &[
            "観測面",
            "プロンプト面",
            "注入面",
            "無認証の面",
            "書き込み面",
            "制御面",
            "管理面",
            "稼働の面",
        ],
        note: Some(
            "計測層が名付ける観測面 ・プロンプト面、露出を指す注入面 ・無認証の面、および「稼働の面」のような側面の意味は別の語で残る。",
        ),
    },
    Rewrite {
        avoid: &["門"],
        write: &["ゲート"],
        when: Some("要求やラウンドが通過すべき検査機能を指す場合"),
        keep: &["専門", "部門", "入門"],
        note: Some("「門」は建物と読める。"),
    },
    Rewrite {
        avoid: &["周"],
        write: &["ループ", "サイクル"],
        when: Some("繰り返し処理の 1 回分を指す場合"),
        keep: &["周期", "周辺", "周知", "円周"],
        note: Some("そのループの周期を指す「周期」は残る。"),
    },
    Rewrite {
        avoid: &["一者"],
        write: &["〜だけ"],
        when: Some("限定を表す場合"),
        keep: &[],
        note: Some("例: 「書き手は supervisor だけ」。"),
    },
    Rewrite {
        avoid: &["断られる"],
        write: &["拒否される"],
        when: Some(
            "拒否するのが人ではなく検査 (finish を拒否するゲート、要求を拒否するハーネス) の場合",
        ),
        keep: &[],
        note: None,
    },
    Rewrite {
        avoid: &["台帳"],
        write: &["テーブル"],
        when: Some("ハーネスがレコード群として管理するファイルを指す場合"),
        keep: &[],
        note: None,
    },
    Rewrite {
        avoid: &["就く", "就ける"],
        write: &["バインドされる", "バインドできる"],
        when: Some("エージェントをタスクへ割り当てることを指す場合"),
        keep: &[],
        note: Some("例: 「評価者はバインドされない」「初めてバインドされる」。"),
    },
    Rewrite {
        avoid: &["束縛する", "束縛される", "割り当てる"],
        write: &["バインドする", "バインドされる"],
        when: Some("エージェントをタスクへ割り当てる動作を指す場合"),
        keep: &[],
        note: Some(
            "登録語「バインド」の動詞形を使う (2026-09-19 操作者裁定)。他の意味の「割り当てる」は普通の語として残る。",
        ),
    },
    Rewrite {
        avoid: &["縁"],
        write: &["エッジ"],
        when: Some("状態遷移図などの接続を指す場合"),
        keep: &[],
        note: None,
    },
    Rewrite {
        avoid: &["回帰"],
        write: &["リグレッション"],
        when: Some("ソフトウェア検証の文脈"),
        keep: &["統計学の回帰", "原文引用に含まれる回帰"],
        note: None,
    },
    Rewrite {
        avoid: &["腕"],
        write: &["群"],
        when: Some("A/B 比較の arm を指す場合"),
        keep: &["Rust の match の腕"],
        note: Some(
            "「A 群」「B 群」「両群」「焦点の群」。arm を含む識別子は残る (2026-09-19 操作者裁定)。",
        ),
    },
    Rewrite {
        avoid: &["族"],
        write: &["タスクグループ"],
        when: Some("親タスクとその子孫の集まりを指す場合"),
        keep: &["qwen3 一族"],
        note: Some(
            "同じ文で Unix のグループを指すときは「Unix グループ」と書いて分ける。識別子 family_root と fam_<root> は残る (2026-09-19 操作者裁定)。",
        ),
    },
    Rewrite {
        avoid: &["referent"],
        write: &["指示対象"],
        when: Some("日本語の文の中"),
        keep: &[],
        note: Some("英語の文の referent は残る (2026-09-19 操作者裁定)。"),
    },
    Rewrite {
        avoid: &["クラウド"],
        write: &["Claude"],
        when: Some("Anthropic のサービス Claude を指す場合"),
        keep: &[],
        note: Some(concat!(
            "サービス名の Claude は常に文字どおり Claude と書き、cloud の「クラウド」と区別する。cloud を指す\n",
            "「クラウド」 (クラウドのセッション、クラウドのコンテナ) は残る。カタカナの「クロード」は条件なしで避ける\n",
            "書き方として src/rules.rs にある (2026-10-21T16:53Z 操作者の用語裁定)。\n",
        )),
    },
    Rewrite {
        avoid: &["口"],
        write: &["ポート"],
        when: Some("network のポート (port 8082 など) を指す場合"),
        keep: &[],
        note: Some(concat!(
            "入口、出口、読み口のような語は残る。network のポートは「口」と呼ばずに「ポート」と書く\n",
            "(2026-10-25T15:52Z 操作者の用語裁定)。\n",
        )),
    },
    Rewrite {
        avoid: &["接続元"],
        write: &["ソースIPレンジ"],
        when: Some("ポートへの接続を許すアドレスの範囲を指す場合"),
        keep: &[],
        note: Some(concat!(
            "「この口の接続元」ではなく「このポートへの接続を許すソースIPレンジ」と書く。実際に接続してきた個々の\n",
            "ホストを数えた結果は、送信元の IP アドレスと書く (2026-10-25T15:52Z 操作者の用語裁定)。\n",
        )),
    },
    Rewrite {
        avoid: &["偽の"],
        write: &["モックの", "テスト用の"],
        when: Some("テストの中で本物の代わりに置く代役 (モック、スタブ) を指す場合"),
        keep: &["真偽の"],
        note: Some(concat!(
            "「偽の」は悪意を含む語なので、テストの代役には使わない (2026-10-26T13:58Z 操作者の用語裁定)。\n",
            "偽造された署名や承認、真偽値の偽、誤って出た赤のような別の意味は、この記録の対象の外である。\n",
        )),
    },
    Rewrite {
        avoid: &["触れる", "触れない"],
        write: &["端点に持つ", "つながる"],
        when: Some("グラフの辺とノードの関係を言う場合"),
        keep: &[],
        note: Some(concat!(
            "「取り下げたノードに触れる辺」は意味が通じない (2026-10-26T14:00Z 操作者の用語裁定)。「取り下げたノードを\n",
            "端点に持つ辺」と書く。ファイルや話題に触れるのような一般の意味は、この記録の対象の外である。\n",
        )),
    },
    Rewrite {
        avoid: &["現役"],
        write: &["有効な"],
        when: Some("作業グラフのノードや辺が取り下げも完了もされていない状態を言う場合"),
        keep: &[],
        note: Some(concat!(
            "削除されていないノードを「現役」と呼ぶのは不自然 (2026-10-26T14:03Z 操作者の用語裁定)。「有効なノード」と\n",
            "書く。「アクティブ」も裁定は認めるが、状態値 `active` と紛れるので、作業グラフでは「有効」を使う。\n",
        )),
    },
    Rewrite {
        avoid: &["印"],
        write: &["マーク"],
        when: Some("英語の mark や marker を訳して、表示や判定のマークを指す場合"),
        keep: &["矢印", "印刷", "印象", "目印"],
        note: Some(concat!(
            "英語の技術用語を漢字の造語へ無理に訳さず、定着したカタカナ語で書く (2026-10-26T15:32Z 操作者の\n",
            "用語裁定)。「着手可の印」は「着手可のマーク」と書く。\n",
        )),
    },
    Rewrite {
        avoid: &["型"],
        write: &["タイプ"],
        when: Some("グラフのノードや辺のような、データの要素の種類を識別する値を指す場合"),
        keep: &["型紙", "類型", "典型", "原型"],
        note: Some(concat!(
            "プログラミングで「型」はプログラミング言語上の値の型を指すので、データの要素の種類を識別する値を\n",
            "「型」と呼ぶと紛れる (2026-10-27T10:50Z 操作者の用語裁定)。「辺の型 `replaces`」は「辺のタイプ\n",
            "`replaces`」と書く。Rust の型や戻り値の型のような、プログラミング言語上の値の型は、この記録の対象の外である。\n",
        )),
    },
    Rewrite {
        avoid: &["上書き"],
        write: &["例外として扱う"],
        when: Some("判定や規則が拒否を通すことを言い、何も書き込まない場合"),
        keep: &[],
        note: Some(concat!(
            "何も書き込まない操作を「上書き」と呼ぶのは不適切 (2026-10-27T11:04Z 操作者の用語裁定)。「反復の判定で\n",
            "拒否を上書きして閉じる」は「反復の判定で拒否を例外として扱って閉じる」と書く。ファイルや値を書き換える\n",
            "本来の上書きは、この記録の対象の外である。\n",
        )),
    },
    Rewrite {
        avoid: &["余り"],
        write: &["ファイルに無く、グラフにだけあるノードと辺"],
        when: Some("何の余りかを書かずに、単独で「余り」と書く場合"),
        keep: &[],
        note: Some(concat!(
            "「割り算の余り」「材料の余り」のように何の余りかを書けば使えるが、単独の「余り」は何の余りかが\n",
            "通じない (2026-10-26T15:31Z 操作者の用語裁定)。何が何に対して残っているのかを書く。\n",
        )),
    },
    Rewrite {
        avoid: &["組む", "組んだ", "組み直す", "組み直した", "組み直し"],
        write: &[
            "ビルドする",
            "ビルドした",
            "ビルドし直す",
            "ビルドし直した",
            "再ビルド",
        ],
        when: Some("ソースからバイナリ、イメージ、成果物をビルドすることを指す場合"),
        keep: &[],
        note: Some(concat!(
            "「組み直した」は「ビルドし直した」と書く (2026-10-25T16:44Z 操作者の用語裁定)。文、計画、プロンプトを\n",
            "「組み直す」のような別の意味の語は残る。バイナリとイメージに続く並びは src/rules.rs が条件なしで検査する。\n",
        )),
    },
    Rewrite {
        avoid: &["作った"],
        write: &["作成した"],
        when: Some("修正、コミット、ブランチ、PR、ファイルのような成果物を作ったことを述べる場合"),
        keep: &[],
        note: Some(concat!(
            "「作った直し」は「作成した修正」と書く (2026-10-25T16:44Z 操作者の用語裁定)。名詞の「直し」の側は\n",
            "src/rules.rs が条件なしで検査する。\n",
        )),
    },
];

// ---------------------------------------------------------------------------------------------
// 一般語がその一般的な意味で使われ、登録語と衝突しないと判定した例
// ---------------------------------------------------------------------------------------------

pub static ORDINARY: &[Ordinary] = &[
    Ordinary {
        word: "reply",
        note: concat!(
            "chat の instruction にある操作者向けの発話の reply と、同じ語を持つソースのモジュールとパーサとは、別の対象を指すので、同じ語を使ってよい。\n",
            "どちらも「応えて何かを返す」という普通の意味で、木が分けて保つ対象を指していない。「既に使われている」\n",
            "という語面で読むレビューは書き手が選べるほぼ全ての名前に当たる。決めるのは指示対象である。\n",
        ),
    },
    Ordinary {
        word: "ステップ (手順の段階)",
        note: concat!(
            "「9 ステップの手順」「9 段階」のように手順の段階を数える一般語のステップと、LLM 呼び出しの単位として\n",
            "登録された「ステップ」とは、別の対象を指すので、同じ語を使ってよい。登録が縛るのは構成要素の名前であり、一般語を避けるために不自然な\n",
            "漢語を作らない (2026-09-19 操作者裁定)。\n",
        ),
    },
    Ordinary {
        word: "表 (Markdown の表)",
        note: "文書の中の表は「表」と書く。登録語「テーブル」はハーネスがレコード群として持つファイルを指す。",
    },
];

// ---------------------------------------------------------------------------------------------
// 標準語と認めた略語 (must/no-unexplained-abbreviations の「読み手が既に知る標準の語」)
// ---------------------------------------------------------------------------------------------

pub static ABBREVIATION: &[Abbreviation] = &[
    Abbreviation {
        word: "dir",
        note: concat!(
            "directory の略。Unix のコマンド名 (mkdir、rmdir) と std::fs::read_dir がこの書き方を持ち、このコードベースを\n",
            "見たことのない読み手が展開せずに読む。task_dir、src_dir のような接尾語にも使う (2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "args",
        note: concat!(
            "arguments の略。std::env::args と clap の慣用がこの書き方を持ち、argv と同じく読み手が展開せずに読む\n",
            "(2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "arg",
        note: "argument の略。args の単数で、関数と clap の引数名の慣用 (2026-09-22 操作者裁定)。",
    },
    Abbreviation {
        word: "src",
        note: concat!(
            "source の略。Rust の src/ ディレクトリと、コピーや転送の (src, dst) の対がこの書き方を持つ\n",
            "(2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "dst",
        note: concat!(
            "destination の略。src と対で使う書き方で、libc の memcpy や std::ptr::copy の引数名でもある\n",
            "(2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "cmd",
        note: "command の略。std::process::Command を組む変数名の慣用で、読み手が展開せずに読む (2026-09-22 操作者裁定)。\n",
    },
    Abbreviation {
        word: "Cmd",
        note: concat!(
            "command の略の型名 (Cmd、HostCmd、BenchCmd、WorktreeCmd)。clap のサブコマンド enum の名前に使う\n",
            "(2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "tmp",
        note: concat!(
            "temporary の略。`/tmp` を指さない局所名 (一時変数、一時ファイル) としての tmp も、/tmp と同じ書き方として\n",
            "読み手が展開せずに読む。効く範囲は局所名に限る (2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "msg",
        note: concat!(
            "message の略。syslog や各種メッセージングの API がこの書き方を持ち、読み手が展開せずに読む\n",
            "(2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "op",
        note: concat!(
            "operation の略。op コードと op 文字列の慣用がある。ワイヤの op 文字列を映す識別子は契約でもある\n",
            "(2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "ops",
        note: "operation の複数形の略。op と同じ理由で標準語 (2026-09-22 操作者裁定)。",
    },
    Abbreviation {
        word: "resp",
        note: concat!(
            "response の略。HTTP クライアントの変数名の慣用で、読み手が展開せずに読む。req の側は request へ書き出したが、\n",
            "resp は標準語として残す (2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "Resp",
        note: "response の略の型名 (http.rs の応答型)。resp と同じ理由で標準語 (2026-09-22 操作者裁定)。",
    },
    Abbreviation {
        word: "spec",
        note: "specification の略。辞書語として通り、型名 (Spec、SubtaskSpec、MarkSpec) にも使う (2026-09-22 操作者裁定)。\n",
    },
    Abbreviation {
        word: "secs",
        note: "seconds の略。std::time::Duration の from_secs と as_secs が持つ書き方 (2026-09-22 操作者裁定)。\n",
    },
    Abbreviation {
        word: "ts",
        note: "timestamp の略。ログと時系列のフィールド名として広く通る (2026-09-22 操作者裁定)。",
    },
    Abbreviation {
        word: "opts",
        note: "options の略 (SyncOpts、SpawnOpts、RenderOpts、DeleteOpts、TaskOpts)。getopt 系の慣用 (2026-09-22 操作者裁定)。\n",
    },
    Abbreviation {
        word: "prev",
        note: "previous の略。prev / next の対として反復と連結リストで広く通る (2026-09-22 操作者裁定)。",
    },
    Abbreviation {
        word: "num",
        note: "number の略。num_ctx のような契約名の接頭語でもあり、読み手が展開せずに読む (2026-09-22 操作者裁定)。\n",
    },
    Abbreviation {
        word: "sig",
        note: "signature の略 (sig / sigs)。ssh-keygen -Y sign が出す .sig 拡張子と同じ書き方 (2026-09-22 操作者裁定)。\n",
    },
    Abbreviation {
        word: "ident",
        note: concat!(
            "identifier の略。Rust の proc_macro::Ident と syn::Ident が持つ書き方で、module ident.rs もこの語\n",
            "(2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "dev",
        note: concat!(
            "device と development の略。grant 名 \"dev\" と dev-base の overlay 名は契約で、それ以外の局所名も同じ書き方の\n",
            "まま読める。assistant は箇所ごとに development / device へ書き出す案を出したが、採らなかった\n",
            "(2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "stat",
        note: concat!(
            "statistics の略 (cdp.rs の stat 関数と Stat 型)。quota.rs の statvfs を指す stat は元から標準語。assistant は\n",
            "statistics へ書き出す案を出したが、採らなかった (2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "gen",
        note: concat!(
            "generation の略 (benchreport.rs の `gen: Vec<f64>`。finish 行の gen_s を集めた生成時間の系列)。assistant は\n",
            "書き出す案を出したが、採らなかった (2026-09-22 操作者裁定)。\n",
        ),
    },
    Abbreviation {
        word: "sel",
        note: concat!(
            "selector と selection の略 (lint.rs の CSS セレクタと、dashboard.rs の HTML の抜き出しの局所名)。assistant は箇所ごとに selector /\n",
            "selection へ書き出す案を出したが、採らなかった (2026-09-22 操作者裁定)。\n",
        ),
    },
];

#[cfg(test)]
mod tests {
    //! 表の形と検査規則との整合を検証する。

    use super::*;
    use crate::rules::{RULES, Rule};
    use std::collections::HashSet;

    fn blank(s: &str) -> bool {
        s.trim().is_empty()
    }

    /// 語の記録だけを対象に、形の違反を列挙する。空なら well-formed。
    fn violations(
        register: &[Register],
        rewrite: &[Rewrite],
        ordinary: &[Ordinary],
        abbreviation: &[Abbreviation],
    ) -> Vec<String> {
        violations_with_rules(register, rewrite, &[], ordinary, abbreviation)
    }

    /// 形の違反を列挙する。検査規則 (src/rules.rs) の avoid も、語の記録と合わせて見る:
    /// 同じ avoid を両方の表に持たないこと、登録語を条件なしで避ける規則が無いこと。
    fn violations_with_rules(
        register: &[Register],
        rewrite: &[Rewrite],
        rules: &[Rule],
        ordinary: &[Ordinary],
        abbreviation: &[Abbreviation],
    ) -> Vec<String> {
        let mut out = Vec::new();
        if register.is_empty() && rewrite.is_empty() {
            out.push("表に項が 1 つも無い".to_string());
        }
        let mut registered: HashSet<&str> = HashSet::new();
        for (i, r) in register.iter().enumerate() {
            let n = i + 1;
            if blank(r.word) {
                out.push(format!("register 項 {n}: word が空"));
            } else if !registered.insert(r.word) {
                out.push(format!(
                    "register 項 {n}: word「{}」が別の項にもある",
                    r.word
                ));
            }
            if blank(r.meaning) {
                out.push(format!("register 項 {n}: meaning が空"));
            }
            if matches!(r.note, Some(s) if blank(s)) {
                out.push(format!(
                    "register 項 {n}: note が空文字 (補足が無いなら欄ごと省く)"
                ));
            }
            for list in [r.ident, r.not, r.verb] {
                if list.iter().any(|s| blank(s)) {
                    out.push(format!("register 項 {n}: 配列に空文字"));
                }
            }
        }
        let mut avoided: HashSet<&str> = HashSet::new();
        for (i, r) in rewrite.iter().enumerate() {
            let n = i + 1;
            if r.avoid.is_empty() {
                out.push(format!("rewrite 項 {n}: avoid が空"));
            }
            if r.write.is_empty() {
                out.push(format!("rewrite 項 {n}: write が空"));
            }
            for a in r.avoid {
                if blank(a) {
                    out.push(format!("rewrite 項 {n}: avoid に空文字"));
                    continue;
                }
                if !avoided.insert(a) {
                    out.push(format!(
                        "rewrite 項 {n}: avoid の書き方「{a}」が別の項にもある"
                    ));
                }
                if registered.contains(a) && r.when.is_none() {
                    out.push(format!(
                        "rewrite 項 {n}: 登録語「{a}」を条件なしで avoid にしている (when を書く)"
                    ));
                }
                if r.keep.contains(a) {
                    out.push(format!("rewrite 項 {n}: keep に avoid と同じ書き方「{a}」"));
                }
            }
            if r.write.iter().any(|w| blank(w)) {
                out.push(format!("rewrite 項 {n}: write に空文字"));
            }
            if matches!(r.when, Some(s) if blank(s)) {
                out.push(format!(
                    "rewrite 項 {n}: when が空文字 (条件が無いなら欄ごと省く)"
                ));
            }
            if matches!(r.note, Some(s) if blank(s)) {
                out.push(format!(
                    "rewrite 項 {n}: note が空文字 (補足が無いなら欄ごと省く)"
                ));
            }
        }
        // 検査規則の avoid は常に効くため、登録語を保護し、語の記録の avoid との重複を検出する
        // (語の記録を 1 か所に保つ)
        for (i, r) in rules.iter().enumerate() {
            let n = i + 1;
            for a in r.avoid {
                if blank(a) {
                    out.push(format!("rule 項 {n}: avoid に空文字"));
                    continue;
                }
                if !avoided.insert(a) {
                    out.push(format!(
                        "rule 項 {n}: avoid の書き方「{a}」が語の記録か別の規則にもある"
                    ));
                }
                if registered.contains(a) {
                    out.push(format!(
                        "rule 項 {n}: 登録語「{a}」を条件なしで避ける規則にしている (条件付きなら words.rs の REWRITE に when を書いて置く)"
                    ));
                }
                if r.keep.contains(a) {
                    out.push(format!("rule 項 {n}: keep に avoid と同じ書き方「{a}」"));
                }
            }
        }
        let mut ordinary_words: HashSet<&str> = HashSet::new();
        for (i, o) in ordinary.iter().enumerate() {
            let n = i + 1;
            if blank(o.word) {
                out.push(format!("ordinary 項 {n}: word が空"));
            } else if !ordinary_words.insert(o.word) {
                out.push(format!(
                    "ordinary 項 {n}: word「{}」が別の項にもある",
                    o.word
                ));
            }
            if blank(o.note) {
                out.push(format!("ordinary 項 {n}: note が空"));
            }
        }
        let mut abbreviations: HashSet<&str> = HashSet::new();
        for (i, a) in abbreviation.iter().enumerate() {
            let n = i + 1;
            if blank(a.word) {
                out.push(format!("abbreviation 項 {n}: word が空"));
            } else if !abbreviations.insert(a.word) {
                out.push(format!(
                    "abbreviation 項 {n}: word「{}」が別の項にもある",
                    a.word
                ));
            }
            if blank(a.note) {
                out.push(format!("abbreviation 項 {n}: note が空"));
            }
        }
        out
    }

    fn reg(word: &'static str, meaning: &'static str) -> Register {
        Register {
            word,
            ident: &[],
            meaning,
            not: &[],
            verb: &[],
            note: None,
        }
    }

    fn rw(avoid: &'static [&'static str], write: &'static [&'static str]) -> Rewrite {
        Rewrite {
            avoid,
            write,
            when: None,
            keep: &[],
            note: None,
        }
    }

    fn rule(avoid: &'static [&'static str]) -> Rule {
        Rule {
            avoid,
            write: &["w"],
            keep: &[],
            note: None,
            bad: &[],
            good: &[],
        }
    }

    #[test]
    fn the_word_tables_are_well_formed() {
        let v = violations_with_rules(REGISTER, REWRITE, RULES, ORDINARY, ABBREVIATION);
        assert!(
            v.is_empty(),
            "words.rs に {} 件の違反:\n  {}",
            v.len(),
            v.join("\n  ")
        );
        assert!(!REGISTER.is_empty() && !ORDINARY.is_empty());
        assert!(
            !ABBREVIATION.is_empty(),
            "2026-09-22 の裁定で標準語と認めた略語の表が空"
        );
    }

    #[test]
    fn a_rule_avoiding_a_registered_word_is_reported() {
        let v = violations_with_rules(
            &[reg("ラウンド", "区間")],
            &[rw(&["席"], &["エージェント"])],
            &[rule(&["ラウンド"])],
            &[],
            &[],
        );
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("rule 項 1: 登録語「ラウンド」"), "{v:?}");
    }

    #[test]
    fn an_avoid_held_by_both_the_word_record_and_a_rule_is_reported() {
        let v = violations_with_rules(
            &[],
            &[rw(&["配備"], &["デプロイ"])],
            &[rule(&["配備"]), rule(&["基線", "基線"])],
            &[],
            &[],
        );
        assert_eq!(v.len(), 2, "{v:?}");
        assert!(v[0].contains("rule 項 1: avoid の書き方「配備」"), "{v:?}");
        assert!(v[1].contains("rule 項 2: avoid の書き方「基線」"), "{v:?}");
        let t = Rule {
            keep: &["窓"],
            ..rule(&["窓", " "])
        };
        let v = violations_with_rules(&[], &[rw(&["席"], &["x"])], &[t], &[], &[]);
        assert!(
            v.iter().any(|s| s.contains("rule 項 1: keep に avoid")),
            "{v:?}"
        );
        assert!(
            v.iter().any(|s| s.contains("rule 項 1: avoid に空文字")),
            "{v:?}"
        );
    }

    #[test]
    fn empty_tables_are_reported() {
        let v = violations(&[], &[], &[], &[]);
        assert_eq!(v, vec!["表に項が 1 つも無い".to_string()]);
    }

    #[test]
    fn a_duplicate_avoid_spelling_is_reported() {
        let v = violations(
            &[],
            &[rw(&["席"], &["エージェント"]), rw(&["席"], &["スロット"])],
            &[],
            &[],
        );
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("別の項にもある"), "{v:?}");
    }

    #[test]
    fn an_empty_write_and_a_keep_equal_to_avoid_are_reported() {
        let t = Rewrite {
            keep: &["束"],
            ..rw(&["束"], &[])
        };
        let v = violations(&[], &[t], &[], &[]);
        assert!(v.iter().any(|s| s.contains("write が空")), "{v:?}");
        assert!(
            v.iter().any(|s| s.contains("keep に avoid と同じ書き方")),
            "{v:?}"
        );
    }

    #[test]
    fn an_empty_avoid_or_blank_strings_are_reported() {
        let v = violations(&[], &[rw(&[], &["x"]), rw(&[" "], &[""])], &[], &[]);
        assert!(
            v.iter().any(|s| s.contains("rewrite 項 1: avoid が空")),
            "{v:?}"
        );
        assert!(
            v.iter().any(|s| s.contains("rewrite 項 2: avoid に空文字")),
            "{v:?}"
        );
        assert!(
            v.iter().any(|s| s.contains("rewrite 項 2: write に空文字")),
            "{v:?}"
        );
    }

    #[test]
    fn an_empty_when_or_note_is_reported() {
        let t = Rewrite {
            when: Some(""),
            note: Some(" "),
            ..rw(&["配備"], &["デプロイ"])
        };
        let v = violations(&[], &[t], &[], &[]);
        assert!(v.iter().any(|s| s.contains("when が空文字")), "{v:?}");
        assert!(v.iter().any(|s| s.contains("note が空文字")), "{v:?}");
    }

    #[test]
    fn a_registered_word_avoided_without_a_condition_is_reported() {
        let v = violations(
            &[reg("ラウンド", "区間")],
            &[rw(&["ラウンド"], &["生成"])],
            &[],
            &[],
        );
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("条件なしで avoid"), "{v:?}");
        let ok = Rewrite {
            when: Some("生成の意味"),
            ..rw(&["ラウンド"], &["生成"])
        };
        assert!(violations(&[reg("ラウンド", "区間")], &[ok], &[], &[]).is_empty());
    }

    #[test]
    fn an_empty_register_field_is_reported() {
        let t = Register {
            ident: &[""],
            note: Some(""),
            ..reg(" ", "")
        };
        let v = violations(&[t], &[], &[], &[]);
        assert!(
            v.iter().any(|s| s.contains("register 項 1: word が空")),
            "{v:?}"
        );
        assert!(
            v.iter().any(|s| s.contains("register 項 1: meaning が空")),
            "{v:?}"
        );
        assert!(
            v.iter().any(|s| s.contains("register 項 1: note が空文字")),
            "{v:?}"
        );
        assert!(
            v.iter().any(|s| s.contains("register 項 1: 配列に空文字")),
            "{v:?}"
        );
    }

    #[test]
    fn a_duplicate_registered_word_and_an_empty_ordinary_note_are_reported() {
        let v = violations(
            &[reg("タスク", "作業"), reg("タスク", "作業")],
            &[],
            &[Ordinary {
                word: "reply",
                note: "",
            }],
            &[],
        );
        assert!(
            v.iter()
                .any(|s| s.contains("word「タスク」が別の項にもある")),
            "{v:?}"
        );
        assert!(
            v.iter().any(|s| s.contains("ordinary 項 1: note が空")),
            "{v:?}"
        );
    }

    #[test]
    fn a_duplicate_or_empty_ordinary_word_is_reported() {
        let o = |word, note| Ordinary { word, note };
        let v = violations(
            &[reg("タスク", "作業")],
            &[],
            &[o("reply", "応答"), o("reply", "応答"), o("", "x")],
            &[],
        );
        assert!(
            v.iter()
                .any(|s| s.contains("ordinary 項 2: word「reply」が別の項にもある")),
            "{v:?}"
        );
        assert!(
            v.iter().any(|s| s.contains("ordinary 項 3: word が空")),
            "{v:?}"
        );
    }

    #[test]
    fn a_duplicate_or_empty_abbreviation_is_reported() {
        let a = |word, note| Abbreviation { word, note };
        let v = violations(
            &[reg("タスク", "作業")],
            &[],
            &[],
            &[a("dir", "directory"), a("dir", "directory"), a("", " ")],
        );
        assert!(
            v.iter()
                .any(|s| s.contains("abbreviation 項 2: word「dir」が別の項にもある")),
            "{v:?}"
        );
        assert!(
            v.iter().any(|s| s.contains("abbreviation 項 3: word が空")),
            "{v:?}"
        );
        assert!(
            v.iter().any(|s| s.contains("abbreviation 項 3: note が空")),
            "{v:?}"
        );
    }

    /// 語の記録の文 (meaning、not、note、when) と検査規則の note が、同梱の規則と否定構文の検査に当たらない
    /// こと。避ける語を説明の中で挙げるときは「」で囲む。条件つきの書き方 (REWRITE の avoid) の一部に
    /// 条件なしの規則が当たり、記録と別の案を出す食い違い (「回帰試験」に「試験」が当たった例) も見る。
    #[test]
    fn record_text_passes_the_bundled_checks() {
        use crate::mixed::{Layout, prose_units};
        use crate::scan::{Document, scan};
        let mut texts: Vec<(String, &str)> = Vec::new();
        for r in REGISTER {
            texts.push((format!("register {} meaning", r.word), r.meaning));
            texts.extend(r.note.map(|n| (format!("register {} note", r.word), n)));
            for n in r.not {
                texts.push((format!("register {} not", r.word), n));
            }
        }
        for r in REWRITE {
            texts.extend(r.when.map(|n| (format!("rewrite {:?} when", r.avoid), n)));
            texts.extend(r.note.map(|n| (format!("rewrite {:?} note", r.avoid), n)));
        }
        for o in ORDINARY {
            texts.push((format!("ordinary {}", o.word), o.note));
        }
        for a in ABBREVIATION {
            texts.push((format!("abbreviation {}", a.word), a.note));
        }
        for r in RULES {
            texts.extend(r.note.map(|n| (format!("rule {:?} note", r.avoid), n)));
        }
        let mut found = Vec::new();
        for (key, text) in &texts {
            for h in scan(text, RULES) {
                found.push(format!("{key}: {}", h.render()));
            }
            let units = prose_units(&Document::parse(text), &Layout::plain());
            for h in crate::negative::scan_units(&units) {
                found.push(format!("{key}: {}", h.render()));
            }
        }
        for r in REWRITE {
            for a in r.avoid {
                for h in scan(a, RULES) {
                    found.push(format!(
                        "rewrite の avoid {a}: 規則 {:?} が当たる",
                        h.rule.avoid
                    ));
                }
            }
        }
        assert!(
            found.is_empty(),
            "{} 件:\n  {}",
            found.len(),
            found.join("\n  ")
        );
    }
}
