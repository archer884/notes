# Notes

For searching and viewing inline comments and definitions in markdown-formatted text files.

## What kind of notes?

Like these:

```markdown
"Lackland," he answered at last--and it wasn't entirely a lie. "Rede Lackland."

"Do you think you're strong enough to stand, Mr. Lackland?" said Aria. "If so, perhaps I can show you to a place where you can dress?"

<note tags="character,thorne" comment="Lackland is rather literal; he's been disowned and exiled, so he had no land and no home to call his own. Rede (probably pronounced 'raid') is a shortened form of the name Adelrede. Funnily enough, in this case, it means 'wise,' but it very much isn't.">

It was all he could do to climb the stairs to the room that had been made available for Teddy and himself. By the time he lay down on the lower of the two bunks, he lacked the energy or the interest to get dressed. It seemed the poison had sapped not only his strength but also his will--a fact that became all the more apparent, and all the more trying, when Teddy leapt down from his bunk and began to ask questions.
```

Or definitions like this:

```markdown
Thorne was an outlaw, exiled for crimes against the Crown. He would die an outlaw. At least he would not now die in exile.

<note term="lance" definition="used as a prefix for military ranks or roles in Imperial Army circles. A lance-lieutenant is a cornet in training to become a lieutenant and may act in the lieutenant's place if need be.">

His gray horse gave a snort and an unhappy stamp of its hooves. The mare turned to face back down the trail, the way they had come. Traces of red blood stained the pale chalk of the road, but Thorne lifted his eyes to watch in the distance, and to see what his mount must have heard or smelt. Through the tunnel of light, he beheld three men on horseback. They lashed and spurred their mounts, urging the beasts to speed, and Thorne drew a deep breath that brough fresh pain from the arrow embedded in him. No rest for the wicked, as they say.
```

## How does it work?

The `config` subcommand will configure the program's behavior for your current working directory. Pass it the directory containing your source files.

To get a definition, use `notes define <foo>` where "foo" is the word you're looking for. To view all the notes with a given tag, use `notes search <foo>`, where "foo" is the tag.
