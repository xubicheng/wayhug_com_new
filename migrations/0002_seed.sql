-- 初始内容：三篇双语笔记 + 两件文创（示例条目，站长可在后台直接替换）
INSERT OR IGNORE INTO notes (id, slug, lang, status, published_at, created_at, updated_at, tags)
VALUES (1, 'why-a-slower-site', 'zh', 'published', '2026-10-12', '2026-10-12 09:00:00', '2026-10-12 09:00:00', '读书方法,站点');
INSERT OR IGNORE INTO notes (id, slug, lang, status, published_at, created_at, updated_at, tags)
VALUES (2, 'pencil-in-hand', 'zh', 'published', '2026-10-26', '2026-10-26 09:00:00', '2026-10-26 09:00:00', '读书方法,批注');
INSERT OR IGNORE INTO notes (id, slug, lang, status, published_at, created_at, updated_at, tags)
VALUES (3, 'a-room-of-ones-own', 'zh', 'published', '2026-11-05', '2026-11-05 09:00:00', '2026-11-05 09:00:00', '重读,伍尔夫');

INSERT OR IGNORE INTO note_i18n (note_id, lang, title, summary, body) VALUES (1, 'zh',
'为什么做一个慢一点的读书站',
'我试过把笔记写成产品，最后发现慢下来才写得久。',
'## 先把速度降下来

我试过把读书笔记写成「输出」：每读完一本，整理成一篇结构完整的文章，配图、配摘要、配金句。坚持了三个月，笔记越来越像产品，读书反而变得着急。

后来我把标准降到很低：一次只记一件事——一句让我停下来的话，或者一个我没想明白的问题。写不满三百字也可以发。奇怪的是，这样反而写得久。

> 读书不是把内容搬进脑子，而是让自己有一段时间，和另一个人的想法待在一起。

## 这里会有什么

- 读到的句子，以及它为什么让我停下来；
- 没想明白的问题，允许它一直悬着；
- 偶尔的重读，看看几年后的自己是否还同意当时的批注。

中文和英文各写一份。英文不是翻译，更像重写：换一种语言，很多话必须重新想一遍，这本身也是校对。

## 慢一点的好处

慢下来之后，我开始记得书里那些「没用」的部分：一个比喻，一段题外话，作者突然的犹豫。它们不进论文，也不进摘要，但它们是我真正带走的东西。

这个站点就是为这些部分留的地方。');

INSERT OR IGNORE INTO note_i18n (note_id, lang, title, summary, body) VALUES (1, 'en',
'Why a slower reading site',
'I tried making notes into products. Slowing down is what lasted.',
'## Slowing down first

I once treated reading notes as output: finish a book, then produce a well-structured essay with a summary, a few quotations and a polished ending. I kept it up for three months. The notes began to look like products, and the reading itself became hurried.

So I lowered the bar: record one thing at a time, a sentence that made me stop, or a question I could not answer. Fewer than three hundred words was fine. Strangely, that is what lasted.

> Reading is not moving content into your head. It is spending time with another person''s thinking.

## What will be here

- Sentences I stopped at, and why they stopped me;
- Questions left open, allowed to stay open;
- Occasional rereading, to see whether I still agree with my earlier marginalia.

Each note appears in Chinese and in English. The English is not a translation so much as a rewriting; in another language, many things have to be thought through again, and that is a kind of proofreading.

## The advantage of slow

Once I slowed down, I began to remember the parts of books that are of no use: a metaphor, a digression, a sudden hesitation in the author''s voice. They never reach an essay or a summary, but they are what I actually carry away.

This site is a place for those parts.');

INSERT OR IGNORE INTO note_i18n (note_id, lang, title, summary, body) VALUES (2, 'zh',
'读书时手里要有支笔',
'划线很快，批注很慢；一支笔让阅读留下阻力。',
'## 笔是读得慢的借口

在书页空白处写字，会让人承认自己没读懂。划线很快，批注很慢：划线只需要判断「这句好」，批注要回答「它和我以为的哪里不一样」。

我现在的做法很笨：每本书只带一支铅笔。读到想划线的句子，先在旁边写一个字，比如「疑」「证」「反」。如果一个字都写不出来，就说明我只是被句子的语气打动了，还没有真的理解它。

## 三种批注

1. **疑问**：作者凭什么这么说？他省略了哪一步？
2. **对照**：这句话和我以前读过的哪本书在打架？
3. **经验**：我自己的生活里，有没有一个例子可以放进去？

第三种最难，也最值钱。没有经验支撑的批注，往往只是把作者的话换了个说法。

> 一支笔的作用，不是让书变得更满，而是让阅读留下阻力。阻力会让人慢下来。

## 电子书怎么办

我用电子书时，会把批注导出成纯文本，再手抄一遍到纸上。抄写很浪费时间，但抄的时候，脑子会自动删掉那些其实不重要的话。留下来的，才写进笔记。');

INSERT OR IGNORE INTO note_i18n (note_id, lang, title, summary, body) VALUES (2, 'en',
'A pencil in hand',
'Underlining is quick, annotating is slow. A pencil leaves resistance.',
'## A pencil as an excuse to read slowly

Writing in the margins is a way of admitting that you have not understood. Underlining is quick; annotating is slow. A line only requires the judgement that it is good. A note has to answer: how is this different from what I assumed?

My method is clumsy: one pencil per book. When a sentence makes me want to underline it, I first write a single character beside it: doubt, evidence, objection. If I cannot write even one character, I was probably moved by the tone rather than understanding the argument.

## Three kinds of note

1. **Doubt**: on what grounds does the author say this? Which step was skipped?
2. **Comparison**: which book I have read does this quarrel with?
3. **Experience**: is there an example from my own life that belongs here?

The third is hardest and worth the most. A note without experience often only restates the author in different words.

> A pencil does not make a book fuller. It leaves resistance in the reading, and resistance slows you down.

## What about ebooks

With ebooks, I export the annotations as plain text and copy them out by hand. It wastes time, but while copying, the mind quietly deletes what did not matter. Only what survives goes into a note.');

INSERT OR IGNORE INTO note_i18n (note_id, lang, title, summary, body) VALUES (3, 'zh',
'一间自己的房间，重读',
'书没有变，变的是我愿意停下来的地方。',
'## 重读的尴尬

第一次读《一间自己的房间》是在二十出头，记住的是那句「一个女人如果要写小说，必须有钱，还要有一间自己的房间」。当时把它当成一句口号，觉得只要有一间房，问题就解决了。

十年后重读，才发现伍尔夫真正在意的是更麻烦的东西：时间、教育的积累、被允许犯错的余裕，以及「不被仇恨打断的平静」。房间只是一个起点。

## 被忽略的段落

年轻时的我跳过了书里那些不显眼的句子，比如她写一个女人若想写作，先要有不被怨恨消耗的心力。当时我只关心「女性如何成为作家」，不关心「女性如何被谈论」。

重读的价值，可能就在这里：书没有变，变的是我愿意停下来的地方。

## 批注的对照

我在旧书页上看到十年前的批注，字迹用力，句子很短，多数是「对」「深刻」「必须如此」。现在的批注更长，也更犹豫。我不确定这是变得成熟，还是变得不敢下判断。

也许两者都有。读书留下的痕迹，最后会变成一份自己的思想病历。

## 一句留给以后

如果再过十年重读，我希望自己至少能对今天的判断提出一个具体的问题，而不是只说「当时太天真」。');

INSERT OR IGNORE INTO note_i18n (note_id, lang, title, summary, body) VALUES (3, 'en',
'A Room of One''s Own, reread',
'The book has not changed; where I stop has.',
'## The awkwardness of rereading

I first read A Room of One''s Own in my early twenties. What I remembered was the sentence about a woman needing money and a room of her own if she is to write fiction. I treated it as a slogan: give her a room, and the problem is solved.

Ten years later, rereading, I saw that Woolf cared about something more difficult: time, the accumulation of education, the margin to make mistakes, and the calm that is not interrupted by resentment. The room is only a starting point.

## A passage I skipped

As a young reader I passed over the less conspicuous sentences, such as the observation that a woman who wants to write needs energy that resentment has not consumed. I cared only about how a woman becomes a writer, not about how women are spoken about.

This may be the value of rereading: the book has not changed; what changed is where I am willing to stop.

## Comparing annotations

On the old pages I find notes from ten years ago: heavy strokes, very short sentences, mostly "yes", "profound", "it must be so". My notes now are longer and more hesitant. I cannot tell whether that is maturity or a loss of nerve.

Perhaps both. The traces left by reading eventually become a medical record of one''s own thinking.

## A line for later

If I reread this in another ten years, I hope I can put a concrete question to today''s judgement, rather than simply saying that I was naive.');

INSERT OR IGNORE INTO products (id, slug, price_cents, currency, stock, created_at, updated_at)
VALUES (1, 'linen-notebook', 6800, 'CNY', 'preorder', '2026-10-20 10:00:00', '2026-10-20 10:00:00');
INSERT OR IGNORE INTO product_i18n (product_id, lang, name, description, revenue_note) VALUES (1, 'zh',
'布面读书笔记本 · A5',
'亚麻布封面，100g 米白书写纸，128 页，可完全平摊。内页只印页码与一条很淡的边线，留给批注。',
'售价 68 元。扣除制作、包装与寄送成本后的结余，用于站点服务器、域名与每年两次的赠书。');
INSERT OR IGNORE INTO product_i18n (product_id, lang, name, description, revenue_note) VALUES (1, 'en',
'Linen reading notebook · A5',
'Linen cover, 100 gsm cream paper, 128 pages, lies completely flat. The pages carry only a folio and a faint margin rule, leaving room for notes.',
'Priced at CNY 68. After making, packing and shipping costs, the balance supports the server, the domain, and two book donations each year.');

INSERT OR IGNORE INTO products (id, slug, price_cents, currency, stock, created_at, updated_at)
VALUES (2, 'brass-bookmark', 3900, 'CNY', 'in_stock', '2026-10-20 10:05:00', '2026-10-20 10:05:00');
INSERT OR IGNORE INTO product_i18n (product_id, lang, name, description, revenue_note) VALUES (2, 'zh',
'黄铜书签 · 一枝',
'0.5mm 黄铜，手工打磨，随使用留下包浆。长约 12cm，配棉纸封套。',
'售价 39 元。扣除材料与寄送成本后的结余，并入赠书预算。');
INSERT OR IGNORE INTO product_i18n (product_id, lang, name, description, revenue_note) VALUES (2, 'en',
'Brass bookmark · a single branch',
'0.5 mm brass, finished by hand, developing a patina with use. About 12 cm long, in a cotton-paper sleeve.',
'Priced at CNY 39. After material and shipping costs, the balance goes to the book donation budget.');
