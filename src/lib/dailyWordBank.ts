import type { DailyItem, Level } from "./types";

type WordBankItem = {
  word: string;
  gloss: string;
};

const englishBank: Record<Level, WordBankItem[]> = {
  zero: [
    item("hello", "你好"), item("book", "书"), item("water", "水"), item("friend", "朋友"),
    item("home", "家"), item("food", "食物"), item("day", "一天；白天"), item("name", "名字"),
    item("good", "好的"), item("help", "帮助"), item("school", "学校"), item("family", "家庭"),
  ],
  beginner: [
    item("practice", "练习"), item("curious", "好奇的"), item("useful", "有用的"), item("improve", "提高"),
    item("sentence", "句子"), item("listen", "听"), item("remember", "记住"), item("question", "问题"),
    item("answer", "回答"), item("travel", "旅行"), item("morning", "早晨"), item("because", "因为"),
  ],
  skilled: [
    item("nuance", "细微差别"), item("fluent", "流利的"), item("context", "语境"), item("retain", "记住；保留"),
    item("phrase", "短语"), item("summarize", "总结"), item("accurate", "准确的"), item("contrast", "对比"),
    item("assume", "假设"), item("evidence", "证据"), item("specific", "具体的"), item("transfer", "迁移；转移"),
  ],
  advanced: [
    item("idiomatic", "地道的；惯用的"), item("ambiguity", "歧义"), item("register", "语域"), item("connotation", "隐含意义"),
    item("paraphrase", "改述"), item("subtle", "微妙的"), item("coherence", "连贯性"), item("inference", "推断"),
    item("rhetoric", "修辞"), item("approximate", "近似的"), item("constraint", "约束"), item("interpretation", "解释；理解"),
  ],
};

const translatedBank: Record<string, Record<Level, WordBankItem[]>> = {
  zh: {
    zero: [item("你好", "hello"), item("谢谢", "thank you"), item("水", "water"), item("书", "book"), item("家", "home"), item("饭", "meal"), item("人", "person"), item("名字", "name"), item("好", "good"), item("学校", "school"), item("朋友", "friend"), item("今天", "today")],
    beginner: [item("学习", "learn; study"), item("喜欢", "like"), item("明白", "understand"), item("练习", "practice"), item("问题", "question"), item("回答", "answer"), item("早上", "morning"), item("旅行", "travel"), item("需要", "need"), item("帮助", "help"), item("句子", "sentence"), item("记住", "remember")],
    skilled: [item("语境", "context"), item("表达", "expression"), item("习惯", "habit"), item("记忆", "memory"), item("准确", "accurate"), item("比较", "compare"), item("总结", "summarize"), item("证据", "evidence"), item("具体", "specific"), item("假设", "assumption"), item("转化", "transform"), item("短语", "phrase")],
    advanced: [item("歧义", "ambiguity"), item("隐喻", "metaphor"), item("含义", "connotation"), item("语域", "register"), item("改述", "paraphrase"), item("连贯性", "coherence"), item("推断", "inference"), item("修辞", "rhetoric"), item("约束", "constraint"), item("诠释", "interpretation"), item("近似", "approximation"), item("细微差别", "nuance")],
  },
  ja: {
    zero: [item("こんにちは", "你好"), item("ありがとう", "谢谢"), item("水", "水"), item("本", "书"), item("家", "家"), item("人", "人"), item("名前", "名字"), item("学校", "学校"), item("友達", "朋友"), item("今日", "今天"), item("良い", "好的"), item("食べ物", "食物")],
    beginner: [item("勉強", "学习"), item("好き", "喜欢"), item("分かる", "明白"), item("便利", "方便"), item("意味", "意思"), item("質問", "问题"), item("答え", "回答"), item("練習", "练习"), item("旅行", "旅行"), item("必要", "需要"), item("助ける", "帮助"), item("文", "句子")],
    skilled: [item("文脈", "语境"), item("表現", "表达"), item("習慣", "习惯"), item("記憶", "记忆"), item("正確", "准确"), item("比較", "比较"), item("要約", "总结"), item("証拠", "证据"), item("具体的", "具体的"), item("仮定", "假设"), item("変換", "转化"), item("句", "短语")],
    advanced: [item("曖昧", "歧义"), item("比喩", "隐喻"), item("含意", "隐含意义"), item("語域", "语域"), item("言い換え", "改述"), item("一貫性", "连贯性"), item("推論", "推断"), item("修辞", "修辞"), item("制約", "约束"), item("解釈", "解释"), item("近似", "近似"), item("微妙", "微妙的")],
  },
  ko: {
    zero: [item("안녕하세요", "你好"), item("감사합니다", "谢谢"), item("물", "水"), item("책", "书"), item("집", "家"), item("사람", "人"), item("이름", "名字"), item("학교", "学校"), item("친구", "朋友"), item("오늘", "今天"), item("좋다", "好的"), item("음식", "食物")],
    beginner: [item("공부", "学习"), item("좋아하다", "喜欢"), item("이해하다", "理解"), item("필요", "需要"), item("의미", "意思"), item("질문", "问题"), item("대답", "回答"), item("연습", "练习"), item("여행", "旅行"), item("도움", "帮助"), item("문장", "句子"), item("기억하다", "记住")],
    skilled: [item("맥락", "语境"), item("표현", "表达"), item("습관", "习惯"), item("기억", "记忆"), item("정확한", "准确的"), item("비교", "比较"), item("요약", "总结"), item("증거", "证据"), item("구체적", "具体的"), item("가정", "假设"), item("전환", "转化"), item("구절", "短语")],
    advanced: [item("모호함", "歧义"), item("은유", "隐喻"), item("함의", "隐含意义"), item("어역", "语域"), item("바꿔 말하기", "改述"), item("일관성", "连贯性"), item("추론", "推断"), item("수사", "修辞"), item("제약", "约束"), item("해석", "解释"), item("근사", "近似"), item("미묘함", "微妙")],
  },
  fr: {
    zero: [item("bonjour", "你好"), item("merci", "谢谢"), item("eau", "水"), item("livre", "书"), item("maison", "家"), item("personne", "人"), item("nom", "名字"), item("école", "学校"), item("ami", "朋友"), item("aujourd'hui", "今天"), item("bon", "好的"), item("nourriture", "食物")],
    beginner: [item("apprendre", "学习"), item("aimer", "喜欢"), item("comprendre", "理解"), item("utile", "有用的"), item("question", "问题"), item("réponse", "回答"), item("pratiquer", "练习"), item("voyager", "旅行"), item("besoin", "需要"), item("aider", "帮助"), item("phrase", "句子"), item("mémoire", "记忆")],
    skilled: [item("contexte", "语境"), item("expression", "表达"), item("habitude", "习惯"), item("précis", "准确的"), item("comparer", "比较"), item("résumer", "总结"), item("preuve", "证据"), item("spécifique", "具体的"), item("hypothèse", "假设"), item("transférer", "转移"), item("nuance", "细微差别"), item("formulation", "措辞")],
    advanced: [item("ambiguïté", "歧义"), item("métaphore", "隐喻"), item("connotation", "隐含意义"), item("registre", "语域"), item("paraphrase", "改述"), item("cohérence", "连贯性"), item("inférence", "推断"), item("rhétorique", "修辞"), item("contrainte", "约束"), item("interprétation", "解释"), item("approximation", "近似"), item("idiomatique", "地道的")],
  },
  de: {
    zero: [item("hallo", "你好"), item("danke", "谢谢"), item("Wasser", "水"), item("Buch", "书"), item("Haus", "家"), item("Mensch", "人"), item("Name", "名字"), item("Schule", "学校"), item("Freund", "朋友"), item("heute", "今天"), item("gut", "好的"), item("Essen", "食物")],
    beginner: [item("lernen", "学习"), item("mögen", "喜欢"), item("verstehen", "理解"), item("nützlich", "有用的"), item("Frage", "问题"), item("Antwort", "回答"), item("üben", "练习"), item("reisen", "旅行"), item("brauchen", "需要"), item("helfen", "帮助"), item("Satz", "句子"), item("merken", "记住")],
    skilled: [item("Kontext", "语境"), item("Ausdruck", "表达"), item("Gewohnheit", "习惯"), item("Gedächtnis", "记忆"), item("genau", "准确的"), item("vergleichen", "比较"), item("zusammenfassen", "总结"), item("Beweis", "证据"), item("spezifisch", "具体的"), item("Annahme", "假设"), item("übertragen", "转移"), item("Wendung", "短语")],
    advanced: [item("Mehrdeutigkeit", "歧义"), item("Metapher", "隐喻"), item("Konnotation", "隐含意义"), item("Register", "语域"), item("Paraphrase", "改述"), item("Kohärenz", "连贯性"), item("Schlussfolgerung", "推断"), item("Rhetorik", "修辞"), item("Einschränkung", "约束"), item("Interpretation", "解释"), item("Annäherung", "近似"), item("idiomatisch", "地道的")],
  },
  es: {
    zero: [item("hola", "你好"), item("gracias", "谢谢"), item("agua", "水"), item("libro", "书"), item("casa", "家"), item("persona", "人"), item("nombre", "名字"), item("escuela", "学校"), item("amigo", "朋友"), item("hoy", "今天"), item("bueno", "好的"), item("comida", "食物")],
    beginner: [item("aprender", "学习"), item("gustar", "喜欢"), item("entender", "理解"), item("útil", "有用的"), item("pregunta", "问题"), item("respuesta", "回答"), item("practicar", "练习"), item("viajar", "旅行"), item("necesitar", "需要"), item("ayudar", "帮助"), item("frase", "句子"), item("recordar", "记住")],
    skilled: [item("contexto", "语境"), item("expresión", "表达"), item("hábito", "习惯"), item("memoria", "记忆"), item("preciso", "准确的"), item("comparar", "比较"), item("resumir", "总结"), item("evidencia", "证据"), item("específico", "具体的"), item("suposición", "假设"), item("transferir", "转移"), item("matiz", "细微差别")],
    advanced: [item("ambigüedad", "歧义"), item("metáfora", "隐喻"), item("connotación", "隐含意义"), item("registro", "语域"), item("paráfrasis", "改述"), item("coherencia", "连贯性"), item("inferencia", "推断"), item("retórica", "修辞"), item("restricción", "约束"), item("interpretación", "解释"), item("aproximación", "近似"), item("idiomático", "地道的")],
  },
};

function item(word: string, gloss: string): WordBankItem {
  return { word, gloss };
}

function dailySeed(language: string, level: Level, rotate: boolean): number {
  const date = new Date().toISOString().slice(0, 10);
  const source = rotate ? `${date}:${language}:${level}:${new Date().getSeconds()}` : `${date}:${language}:${level}`;
  return Array.from(source).reduce((sum, char) => sum + char.charCodeAt(0), 0);
}

function pickDailyWords(language: string, level: Level, rotate: boolean): WordBankItem[] {
  const pool = language === "en" ? englishBank[level] : translatedBank[language]?.[level] ?? englishBank[level];
  const start = dailySeed(language, level, rotate) % pool.length;
  return Array.from({ length: 5 }, (_, index) => pool[(start + index * 3) % pool.length]);
}

function examplesFor(language: string, word: string): { text: string; translation: string }[] {
  if (language === "zh") {
    return [
      { text: `我今天学习“${word}”。`, translation: `I study "${word}" today.` },
      { text: `这个词在句子里很常见。`, translation: "This word is common in sentences." },
      { text: `请用“${word}”造一个句子。`, translation: `Please make a sentence with "${word}".` },
    ];
  }
  if (language === "ja") {
    return [
      { text: `${word}を練習します。`, translation: `练习 ${word}。` },
      { text: "この単語はよく使います。", translation: "这个单词经常使用。" },
      { text: `${word}を例文で覚えます。`, translation: `用例句记住 ${word}。` },
    ];
  }
  if (language === "ko") {
    return [
      { text: `${word}를 연습해요.`, translation: `练习 ${word}。` },
      { text: "이 단어는 자주 써요.", translation: "这个单词经常使用。" },
      { text: `${word}를 예문으로 기억해요.`, translation: `用例句记住 ${word}。` },
    ];
  }
  if (language === "fr") {
    return [
      { text: `J'apprends ${word} aujourd'hui.`, translation: `我今天学习 ${word}。` },
      { text: `${word} apparaît dans des phrases simples.`, translation: `${word} 会出现在简单句子里。` },
      { text: `Je mémorise ${word} avec un exemple.`, translation: `我用例句记住 ${word}。` },
    ];
  }
  if (language === "de") {
    return [
      { text: `Ich lerne ${word} heute.`, translation: `我今天学习 ${word}。` },
      { text: `${word} passt in einfache Sätze.`, translation: `${word} 适合放进简单句子里。` },
      { text: `Ich merke mir ${word} mit einem Beispiel.`, translation: `我用例句记住 ${word}。` },
    ];
  }
  if (language === "es") {
    return [
      { text: `Aprendo ${word} hoy.`, translation: `我今天学习 ${word}。` },
      { text: `${word} aparece en frases simples.`, translation: `${word} 会出现在简单句子里。` },
      { text: `Memorizo ${word} con un ejemplo.`, translation: `我用例句记住 ${word}。` },
    ];
  }
  return [
    { text: `I study ${word} today.`, translation: `我今天学习 ${word}。` },
    { text: `${word} appears in simple sentences.`, translation: `${word} 会出现在简单句子里。` },
    { text: `I remember ${word} with an example.`, translation: `我用例句记住 ${word}。` },
  ];
}

export function dailyFallback(language: string, level: Level, rotate = false): DailyItem[] {
  return pickDailyWords(language, level, rotate).map((word, index) => {
    const examples = examplesFor(language, word.word);
    return {
      id: `${language}-${level}-wordbank-${index}`,
      word: word.word,
      language,
      translation: word.gloss,
      examples: examples.map((example) => example.text),
      exampleTranslations: examples.map((example) => example.translation),
      level,
    };
  });
}
