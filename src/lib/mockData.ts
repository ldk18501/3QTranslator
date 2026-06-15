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

const pools: Record<string, Record<Level, Array<Omit<DailyItem, "id" | "language" | "level">>>> = {
  en: {
    zero: [
      { word: "hello", translation: "你好", examples: ["Hello, my name is Q.", "She said hello with a smile.", "Hello is a friendly first word."] },
      { word: "book", translation: "书", examples: ["This book is easy.", "I read a book every night.", "Put the book on the desk."] },
      { word: "water", translation: "水", examples: ["I drink water.", "The water is cold.", "Please bring some water."] },
      { word: "friend", translation: "朋友", examples: ["He is my friend.", "A good friend listens.", "I met a new friend today."] },
      { word: "home", translation: "家", examples: ["I am going home.", "Home feels warm.", "She works from home."] },
    ],
    beginner: [
      { word: "practice", translation: "练习", examples: ["Practice makes speaking easier.", "I practice English after dinner.", "Daily practice builds confidence."] },
      { word: "curious", translation: "好奇的", examples: ["A curious student asks questions.", "I am curious about this word.", "Curious minds learn faster."] },
      { word: "useful", translation: "有用的", examples: ["This phrase is useful.", "A notebook is useful for study.", "Useful examples help memory."] },
      { word: "improve", translation: "提高", examples: ["I want to improve my listening.", "Small habits improve fluency.", "Feedback helps you improve."] },
      { word: "sentence", translation: "句子", examples: ["Write one sentence.", "This sentence is clear.", "Read the sentence aloud."] },
    ],
    skilled: [
      { word: "nuance", translation: "细微差别", examples: ["The nuance matters in translation.", "She explained the nuance clearly.", "Context reveals nuance."] },
      { word: "fluent", translation: "流利的", examples: ["He became fluent through practice.", "Fluent speech sounds natural.", "She is fluent in three languages."] },
      { word: "context", translation: "语境", examples: ["Context changes the meaning.", "Check the context before translating.", "The word is formal in this context."] },
      { word: "retain", translation: "记住；保留", examples: ["Examples help you retain words.", "The app retains your notes.", "Sleep helps learners retain memory."] },
      { word: "phrase", translation: "短语", examples: ["Learn the whole phrase.", "This phrase sounds natural.", "A phrase can carry culture."] },
    ],
    advanced: [
      { word: "idiomatic", translation: "地道的；惯用的", examples: ["The sentence sounds idiomatic.", "Idiomatic English is hard to translate literally.", "She chose an idiomatic expression."] },
      { word: "ambiguity", translation: "歧义", examples: ["The translator resolved the ambiguity.", "Ambiguity can be useful in poetry.", "Context reduces ambiguity."] },
      { word: "register", translation: "语域", examples: ["Register affects word choice.", "This register is too formal.", "Learners should notice register."] },
      { word: "connotation", translation: "隐含意义", examples: ["The word has a warm connotation.", "Connotation differs from definition.", "Good translators track connotation."] },
      { word: "paraphrase", translation: "改述", examples: ["Paraphrase the idea in simple words.", "A paraphrase can clarify meaning.", "Try to paraphrase after reading."] },
    ],
  },
};

export function dailyFallback(language: string, level: Level): DailyItem[] {
  const pool = pools[language]?.[level] ?? pools.en[level] ?? pools.en.beginner;
  return pool.slice(0, 5).map((item, index) => ({
    ...item,
    id: `${language}-${level}-${index}`,
    language,
    level,
  }));
}
