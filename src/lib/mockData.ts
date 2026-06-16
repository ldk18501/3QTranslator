import type { DailyItem, Definition, Level } from "./types";

export const fallbackDefinitions: Record<string, Definition[]> = {
  salt: [
    {
      partOfSpeech: "noun",
      meaning: "盐；食盐；用于调味或保存食物的晶体物质。",
      example: "Add a little salt before serving.",
      synonyms: ["seasoning", "sodium chloride"],
    },
    {
      partOfSpeech: "verb",
      meaning: "给食物加盐；用盐保存。",
      example: "They salted the fish for winter.",
    },
  ],
  learn: [
    {
      partOfSpeech: "verb",
      meaning: "学习；通过经验或教学获得知识。",
      example: "She wants to learn Japanese this year.",
    },
  ],
};

type DailySeed = {
  word: string;
  translation: string;
  examples: Array<{ text: string; translation: string }>;
};

const englishByLevel: Record<Level, DailySeed[]> = {
  zero: [
    seed("hello", "你好", ["Hello, my name is Q.", "She said hello with a smile.", "Hello is a friendly first word."], ["你好，我叫 Q。", "她微笑着打招呼。", "hello 是一个友好的入门词。"]),
    seed("book", "书", ["This book is easy.", "I read a book every night.", "Put the book on the desk."], ["这本书很简单。", "我每天晚上读一本书。", "把书放在桌子上。"]),
    seed("water", "水", ["I drink water.", "The water is cold.", "Please bring some water."], ["我喝水。", "水是冷的。", "请拿一些水来。"]),
    seed("friend", "朋友", ["He is my friend.", "A good friend listens.", "I met a new friend today."], ["他是我的朋友。", "好朋友会倾听。", "我今天认识了一位新朋友。"]),
    seed("home", "家", ["I am going home.", "Home feels warm.", "She works from home."], ["我要回家。", "家让人感到温暖。", "她在家工作。"]),
  ],
  beginner: [
    seed("practice", "练习", ["Practice makes speaking easier.", "I practice English after dinner.", "Daily practice builds confidence."], ["练习会让口语更容易。", "我晚饭后练习英语。", "每日练习能建立自信。"]),
    seed("curious", "好奇的", ["A curious student asks questions.", "I am curious about this word.", "Curious minds learn faster."], ["好奇的学生会提问。", "我对这个词很好奇。", "好奇的头脑学得更快。"]),
    seed("useful", "有用的", ["This phrase is useful.", "A notebook is useful for study.", "Useful examples help memory."], ["这个短语很有用。", "笔记本对学习有帮助。", "有用的例句有助于记忆。"]),
    seed("improve", "提高", ["I want to improve my listening.", "Small habits improve fluency.", "Feedback helps you improve."], ["我想提高听力。", "小习惯能提高流利度。", "反馈能帮助你进步。"]),
    seed("sentence", "句子", ["Write one sentence.", "This sentence is clear.", "Read the sentence aloud."], ["写一个句子。", "这个句子很清楚。", "把这个句子大声读出来。"]),
  ],
  skilled: [
    seed("nuance", "细微差别", ["The nuance matters in translation.", "She explained the nuance clearly.", "Context reveals nuance."], ["翻译时细微差别很重要。", "她清楚地解释了这个细微差别。", "语境会揭示细微差别。"]),
    seed("fluent", "流利的", ["He became fluent through practice.", "Fluent speech sounds natural.", "She is fluent in three languages."], ["他通过练习变得流利。", "流利的表达听起来自然。", "她能流利使用三种语言。"]),
    seed("context", "语境", ["Context changes the meaning.", "Check the context before translating.", "The word is formal in this context."], ["语境会改变含义。", "翻译前先检查语境。", "这个词在此语境中偏正式。"]),
    seed("retain", "记住；保留", ["Examples help you retain words.", "The app retains your notes.", "Sleep helps learners retain memory."], ["例句帮助你记住单词。", "应用会保留你的笔记。", "睡眠帮助学习者保持记忆。"]),
    seed("phrase", "短语", ["Learn the whole phrase.", "This phrase sounds natural.", "A phrase can carry culture."], ["学习整个短语。", "这个短语听起来很自然。", "短语可以承载文化。"]),
  ],
  advanced: [
    seed("idiomatic", "地道的；惯用的", ["The sentence sounds idiomatic.", "Idiomatic English is hard to translate literally.", "She chose an idiomatic expression."], ["这个句子听起来很地道。", "地道英语很难逐字翻译。", "她选择了一个惯用表达。"]),
    seed("ambiguity", "歧义", ["The translator resolved the ambiguity.", "Ambiguity can be useful in poetry.", "Context reduces ambiguity."], ["译者消除了歧义。", "歧义在诗歌中可能有用。", "语境会减少歧义。"]),
    seed("register", "语域", ["Register affects word choice.", "This register is too formal.", "Learners should notice register."], ["语域会影响选词。", "这种语域太正式了。", "学习者应该注意语域。"]),
    seed("connotation", "隐含意义", ["The word has a warm connotation.", "Connotation differs from definition.", "Good translators track connotation."], ["这个词带有温暖的含义。", "隐含意义不同于定义。", "优秀译者会留意隐含意义。"]),
    seed("paraphrase", "改述", ["Paraphrase the idea in simple words.", "A paraphrase can clarify meaning.", "Try to paraphrase after reading."], ["用简单的话改述这个想法。", "改述可以澄清含义。", "阅读后试着改述。"]),
  ],
};

const languageSeeds: Record<string, DailySeed[]> = {
  zh: [
    seed("学习", "learn; study", ["我每天学习一点新内容。", "学习语言需要耐心。", "她喜欢边听边学习。"], ["I learn a little new content every day.", "Learning a language requires patience.", "She likes learning while listening."]),
    seed("朋友", "friend", ["朋友给了我很多帮助。", "他是我的老朋友。", "我们一起练习口语。"], ["My friend helped me a lot.", "He is my old friend.", "We practice speaking together."]),
    seed("今天", "today", ["今天我想学五个词。", "今天的天气很好。", "今天先复习昨天的内容。"], ["Today I want to learn five words.", "The weather is nice today.", "Review yesterday's content first today."]),
    seed("喜欢", "like", ["我喜欢这门语言。", "她喜欢听慢速音频。", "你喜欢哪个例句？"], ["I like this language.", "She likes listening to slow audio.", "Which example sentence do you like?"]),
    seed("明白", "understand", ["我明白这个句子的意思。", "他还不太明白语法。", "例句能帮助我明白用法。"], ["I understand the meaning of this sentence.", "He does not quite understand the grammar yet.", "Examples help me understand usage."]),
  ],
  ja: [
    seed("こんにちは", "你好", ["こんにちは、はじめまして。", "彼女は笑顔でこんにちはと言いました。", "こんにちはは便利なあいさつです。"], ["你好，初次见面。", "她微笑着说了你好。", "こんにちは 是很实用的问候语。"]),
    seed("勉強", "学习", ["毎日日本語を勉強します。", "勉強は少しずつ続けます。", "例文で単語を勉強します。"], ["我每天学习日语。", "学习要一点点坚持。", "用例句学习单词。"]),
    seed("友達", "朋友", ["友達と会話を練習します。", "彼は大切な友達です。", "新しい友達ができました。"], ["我和朋友练习对话。", "他是重要的朋友。", "我交到了新朋友。"]),
    seed("便利", "方便", ["この表現は便利です。", "便利なアプリを使います。", "短い例文は便利です。"], ["这个表达很方便。", "我使用方便的应用。", "短例句很方便。"]),
    seed("意味", "意思；含义", ["この単語の意味は何ですか。", "文脈で意味が変わります。", "意味を確認しましょう。"], ["这个单词是什么意思？", "含义会随语境改变。", "我们确认一下意思吧。"]),
  ],
  ko: [
    seed("안녕하세요", "你好", ["안녕하세요, 만나서 반가워요.", "그녀는 안녕하세요라고 말했어요.", "안녕하세요는 기본 인사예요."], ["你好，很高兴见到你。", "她说了你好。", "안녕하세요 是基础问候语。"]),
    seed("공부", "学习", ["저는 매일 한국어를 공부해요.", "공부는 꾸준함이 중요해요.", "예문으로 단어를 공부해요."], ["我每天学习韩语。", "学习贵在坚持。", "用例句学习单词。"]),
    seed("친구", "朋友", ["친구와 같이 말하기를 연습해요.", "그는 좋은 친구예요.", "새 친구를 만났어요."], ["我和朋友一起练习口语。", "他是好朋友。", "我认识了新朋友。"]),
    seed("필요", "需要", ["도움이 필요해요.", "이 단어는 자주 필요해요.", "연습이 필요합니다."], ["我需要帮助。", "这个单词经常需要用到。", "需要练习。"]),
    seed("의미", "意思；含义", ["이 문장의 의미를 알아요.", "의미가 조금 달라요.", "문맥이 의미를 설명해요."], ["我知道这个句子的意思。", "意思有点不同。", "语境解释了含义。"]),
  ],
  fr: [
    seed("bonjour", "你好", ["Bonjour, je m'appelle Q.", "Elle dit bonjour avec le sourire.", "Bonjour est une salutation simple."], ["你好，我叫 Q。", "她微笑着打招呼。", "bonjour 是简单的问候语。"]),
    seed("apprendre", "学习", ["J'apprends le français chaque jour.", "Apprendre une langue prend du temps.", "Les exemples aident à apprendre."], ["我每天学习法语。", "学习一门语言需要时间。", "例句有助于学习。"]),
    seed("ami", "朋友", ["Mon ami m'aide à pratiquer.", "Elle parle avec un ami.", "Un bon ami écoute."], ["我的朋友帮我练习。", "她和一位朋友说话。", "好朋友会倾听。"]),
    seed("utile", "有用的", ["Cette phrase est utile.", "Un dictionnaire est utile.", "Les exemples utiles restent en mémoire."], ["这个句子很有用。", "词典很有用。", "有用的例句容易记住。"]),
    seed("comprendre", "理解", ["Je comprends cette phrase.", "Il veut comprendre le contexte.", "Comprendre vient avec la pratique."], ["我理解这个句子。", "他想理解语境。", "理解来自练习。"]),
  ],
  de: [
    seed("hallo", "你好", ["Hallo, ich heiße Q.", "Sie sagt hallo.", "Hallo ist ein einfaches Wort."], ["你好，我叫 Q。", "她说你好。", "hallo 是一个简单的词。"]),
    seed("lernen", "学习", ["Ich lerne jeden Tag Deutsch.", "Wir lernen mit Beispielen.", "Lernen braucht Geduld."], ["我每天学习德语。", "我们用例句学习。", "学习需要耐心。"]),
    seed("Freund", "朋友", ["Mein Freund hilft mir.", "Ein Freund hört zu.", "Ich übe mit einem Freund."], ["我的朋友帮助我。", "朋友会倾听。", "我和朋友一起练习。"]),
    seed("nützlich", "有用的", ["Dieser Satz ist nützlich.", "Das Buch ist nützlich.", "Nützliche Beispiele helfen."], ["这个句子很有用。", "这本书很有用。", "有用的例句会有帮助。"]),
    seed("verstehen", "理解", ["Ich verstehe das Wort.", "Der Kontext hilft beim Verstehen.", "Sie versteht die Frage."], ["我理解这个词。", "语境有助于理解。", "她理解这个问题。"]),
  ],
  es: [
    seed("hola", "你好", ["Hola, me llamo Q.", "Ella dice hola.", "Hola es un saludo común."], ["你好，我叫 Q。", "她说你好。", "hola 是常见问候语。"]),
    seed("aprender", "学习", ["Aprendo español cada día.", "Aprender con ejemplos ayuda.", "Quiero aprender más palabras."], ["我每天学习西班牙语。", "用例句学习有帮助。", "我想学习更多单词。"]),
    seed("amigo", "朋友", ["Mi amigo practica conmigo.", "Un amigo bueno escucha.", "Conocí a un amigo nuevo."], ["我的朋友和我一起练习。", "好朋友会倾听。", "我认识了一位新朋友。"]),
    seed("útil", "有用的", ["Esta frase es útil.", "Un ejemplo útil ayuda.", "La aplicación es útil para estudiar."], ["这个句子很有用。", "有用的例句会有帮助。", "这个应用对学习有用。"]),
    seed("entender", "理解", ["Entiendo la frase.", "El contexto ayuda a entender.", "Ella entiende la palabra."], ["我理解这个句子。", "语境有助于理解。", "她理解这个单词。"]),
  ],
};

const languageWordsByLevel: Record<string, Record<Level, Array<[string, string]>>> = {
  zh: {
    zero: [["你好", "hello"], ["谢谢", "thank you"], ["水", "water"], ["书", "book"], ["家", "home"]],
    beginner: [["学习", "learn; study"], ["朋友", "friend"], ["今天", "today"], ["喜欢", "like"], ["明白", "understand"]],
    skilled: [["语境", "context"], ["表达", "expression"], ["习惯", "habit"], ["记忆", "memory"], ["练习", "practice"]],
    advanced: [["歧义", "ambiguity"], ["隐喻", "metaphor"], ["含义", "connotation"], ["语域", "register"], ["改述", "paraphrase"]],
  },
  ja: {
    zero: [["こんにちは", "你好"], ["ありがとう", "谢谢"], ["水", "水"], ["本", "书"], ["家", "家"]],
    beginner: [["勉強", "学习"], ["友達", "朋友"], ["便利", "方便"], ["意味", "意思"], ["今日", "今天"]],
    skilled: [["文脈", "语境"], ["表現", "表达"], ["習慣", "习惯"], ["記憶", "记忆"], ["練習", "练习"]],
    advanced: [["曖昧", "歧义"], ["比喩", "隐喻"], ["含意", "隐含意义"], ["語域", "语域"], ["言い換え", "改述"]],
  },
  ko: {
    zero: [["안녕하세요", "你好"], ["감사합니다", "谢谢"], ["물", "水"], ["책", "书"], ["집", "家"]],
    beginner: [["공부", "学习"], ["친구", "朋友"], ["필요", "需要"], ["의미", "意思"], ["오늘", "今天"]],
    skilled: [["맥락", "语境"], ["표현", "表达"], ["습관", "习惯"], ["기억", "记忆"], ["연습", "练习"]],
    advanced: [["모호함", "歧义"], ["은유", "隐喻"], ["함의", "隐含意义"], ["어역", "语域"], ["바꿔 말하기", "改述"]],
  },
  fr: {
    zero: [["bonjour", "你好"], ["merci", "谢谢"], ["eau", "水"], ["livre", "书"], ["maison", "家"]],
    beginner: [["apprendre", "学习"], ["ami", "朋友"], ["utile", "有用的"], ["comprendre", "理解"], ["aujourd'hui", "今天"]],
    skilled: [["contexte", "语境"], ["expression", "表达"], ["habitude", "习惯"], ["mémoire", "记忆"], ["pratique", "练习"]],
    advanced: [["ambiguïté", "歧义"], ["métaphore", "隐喻"], ["connotation", "隐含意义"], ["registre", "语域"], ["paraphrase", "改述"]],
  },
  de: {
    zero: [["hallo", "你好"], ["danke", "谢谢"], ["Wasser", "水"], ["Buch", "书"], ["Haus", "家"]],
    beginner: [["lernen", "学习"], ["Freund", "朋友"], ["nützlich", "有用的"], ["verstehen", "理解"], ["heute", "今天"]],
    skilled: [["Kontext", "语境"], ["Ausdruck", "表达"], ["Gewohnheit", "习惯"], ["Gedächtnis", "记忆"], ["Übung", "练习"]],
    advanced: [["Mehrdeutigkeit", "歧义"], ["Metapher", "隐喻"], ["Konnotation", "隐含意义"], ["Register", "语域"], ["Paraphrase", "改述"]],
  },
  es: {
    zero: [["hola", "你好"], ["gracias", "谢谢"], ["agua", "水"], ["libro", "书"], ["casa", "家"]],
    beginner: [["aprender", "学习"], ["amigo", "朋友"], ["útil", "有用的"], ["entender", "理解"], ["hoy", "今天"]],
    skilled: [["contexto", "语境"], ["expresión", "表达"], ["hábito", "习惯"], ["memoria", "记忆"], ["práctica", "练习"]],
    advanced: [["ambigüedad", "歧义"], ["metáfora", "隐喻"], ["connotación", "隐含意义"], ["registro", "语域"], ["paráfrasis", "改述"]],
  },
};

function languageLevelSeeds(language: string, level: Level): DailySeed[] | undefined {
  const words = languageWordsByLevel[language]?.[level];
  if (!words) return languageSeeds[language];

  return words.map(([word, translation]) => {
    if (language === "zh") {
      return seed(word, translation, [`我今天学习“${word}”。`, `这个词在句子里很常见。`, `请用“${word}”造一个句子。`], [`I study "${word}" today.`, "This word is common in sentences.", `Please make a sentence with "${word}".`]);
    }
    if (language === "ja") {
      return seed(word, translation, [`${word}を練習します。`, `この単語はよく使います。`, `${word}を例文で覚えます。`], [`练习 ${word}。`, "这个单词经常使用。", `用例句记住 ${word}。`]);
    }
    if (language === "ko") {
      return seed(word, translation, [`${word}를 연습해요.`, `이 단어는 자주 써요.`, `${word}를 예문으로 기억해요.`], [`练习 ${word}。`, "这个单词经常使用。", `用例句记住 ${word}。`]);
    }
    if (language === "fr") {
      return seed(word, translation, [`J'apprends ${word} aujourd'hui.`, `${word} apparaît dans des phrases simples.`, `Je mémorise ${word} avec un exemple.`], [`我今天学习 ${word}。`, `${word} 会出现在简单句子里。`, `我用例句记住 ${word}。`]);
    }
    if (language === "de") {
      return seed(word, translation, [`Ich lerne ${word} heute.`, `${word} passt in einfache Sätze.`, `Ich merke mir ${word} mit einem Beispiel.`], [`我今天学习 ${word}。`, `${word} 适合放进简单句子里。`, `我用例句记住 ${word}。`]);
    }
    return seed(word, translation, [`Aprendo ${word} hoy.`, `${word} aparece en frases simples.`, `Memorizo ${word} con un ejemplo.`], [`我今天学习 ${word}。`, `${word} 会出现在简单句子里。`, `我用例句记住 ${word}。`]);
  });
}

function seed(word: string, translation: string, examples: string[], translations: string[]): DailySeed {
  return {
    word,
    translation,
    examples: examples.map((text, index) => ({ text, translation: translations[index] ?? "" })),
  };
}

export function dailyFallback(language: string, level: Level, rotate = false): DailyItem[] {
  const pool = language === "en" ? englishByLevel[level] : languageLevelSeeds(language, level) ?? englishByLevel[level];
  const offset = rotate && pool.length > 0 ? new Date().getSeconds() % pool.length : 0;
  return Array.from({ length: Math.min(5, pool.length) }, (_, index) => pool[(index + offset) % pool.length]).map((item, index) => ({
    id: `${language}-${level}-v2-${index}`,
    word: item.word,
    language,
    translation: item.translation,
    examples: item.examples.map((example) => example.text),
    exampleTranslations: item.examples.map((example) => example.translation),
    level,
  }));
}
