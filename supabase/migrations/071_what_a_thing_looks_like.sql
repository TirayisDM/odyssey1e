-- 071. WHAT A THING LOOKS LIKE.
--
-- `items.description` has existed since 004 and has been NULL on all 89
-- catalogue rows for every one of those migrations. 070 taught the
-- sheet to show it, which turned a quiet absence into a visible one:
-- an item panel with Weight, Size and Value filled in and the line
-- above them blank.
--
-- THESE SAY WHAT A THING IS, NOT WHAT IT DOES. The numbers are already
-- on screen beside the text and a description that repeats them is
-- noise - so none of these names a die, a price or a pound. They say
-- what the object looks like in a hand, and, where there is one, why
-- the thing exists at all: a guisarme is a pruning hook that learned to
-- pull riders down, and that is more use at the table than "1d4
-- slashing, reach" said twice.
--
-- GLOBAL ROWS ONLY. `game_id IS NULL` is the SRD catalogue every game
-- reads - see the nullable-tenancy note in the README. A campaign that
-- has reskinned an item into its own row keeps whatever it wrote.
--
-- NOT AN OVERWRITE. The `description IS NULL` guard means this can be
-- re-run and will not undo authoring done after it. The only custom
-- entry in the list, `mace_of_the_deep_song`, is Dave's and is
-- described in the same register as the rest rather than given a
-- history it has not earned yet.

update items as i
set description = v.d
from (values
  /* ---------------------------- armour ---------------------------- */
  ('breastplate', $d$A fitted steel cuirass over a leather backing, leaving the arms and legs free. Worn by those who need to be seen as much as protected.$d$),
  ('chain_mail', $d$Interlocking rings over a quilted coat, collar to knee, with gauntlets and a mail hood. It rings like a sack of coins at every step.$d$),
  ('chain_shirt', $d$A shirt of fine rings worn under ordinary clothing. Quiet enough to pass for a merchant and sound enough to survive being wrong about one.$d$),
  ('half_plate', $d$Shaped plates over a mail shirt - breast, shoulders, thighs - with the joints left in mail. Everything a full harness covers except the legs below the knee.$d$),
  ('hide', $d$Thick furs and cured hides sewn in overlapping layers. Crude, warm, and made wherever there are animals and no forge.$d$),
  ('leather', $d$Boiled in oil until it holds its shape, then cut to the chest and shoulders. Light, silent, and the first armour most adventurers own.$d$),
  ('padded', $d$Layered quilted cloth, stuffed and stitched into a heavy coat. It turns a blade more often than it looks like it should, and it rustles at every movement.$d$),
  ('plate', $d$A full harness of shaped steel, every piece articulated and fitted to one wearer. Months of a master smith''s work, and it shows.$d$),
  ('ring_mail', $d$Heavy leather sewn with overlapping steel rings. Cheaper than mail and looks it - the armour of gate guards and garrisons.$d$),
  ('scale_mail', $d$Overlapping metal scales on a leather backing, with coat, gauntlets and leggings. Protective, and loud.$d$),
  ('shield', $d$Banded wood faced with hide or thin steel, gripped at the boss. Held well, it is worth more than any coat of armour.$d$),
  ('splint', $d$Vertical strips of metal riveted to leather over a mail underlayer. Rigid, punishing to march in, and very hard to cut through.$d$),
  ('studded_leather', $d$Tough hardened leather set with close-packed rivets. The studs turn a point that plain leather would let through.$d$),

  /* -------------------------- consumable -------------------------- */
  ('holy_water', $d$A flask of water blessed at a shrine, stoppered with wax and marked with the sign of the house that blessed it. Harmless to the living.$d$),
  ('rations', $d$A day of dry travel food - hard biscuit, cured meat, dried fruit and a little salt. Sustaining, and nothing more.$d$),

  /* --------------------------- containers -------------------------- */
  ('backpack', $d$A canvas sack on shoulder straps with a drawstring throat and side loops for a bedroll. Everything a traveller owns ends up in one.$d$),
  ('chest', $d$A banded wooden strongbox with iron corners and a hasp for a lock. Heavy when empty, and it is seldom empty.$d$),
  ('coin_purse', $d$A small leather pouch on a drawstring, worn at the belt or hidden under it. The second is the wiser choice in a crowd.$d$),
  ('priests_pack', $d$A travelling kit for a cleric on the road - blanket, candles, tinderbox, alms box, incense and censer, vestments, food and water.$d$),
  ('quiver', $d$A stiffened leather tube for twenty arrows, slung at the hip or across the back.$d$),
  ('spell_book', $d$A heavy leather-bound volume of written and blank vellum, held shut by a clasp. A wizard''s whole craft, and the one thing they will run back into a burning house for.$d$),

  /* --------------------------- equipment --------------------------- */
  ('clothes_fine', $d$Well-cut cloth with proper tailoring - the kind of garment that gets a stranger through a door. Impractical to fight in and ruinous to replace.$d$),
  ('lamp', $d$A brass oil lamp with a shuttered wick, throwing light further than a candle and in one direction.$d$),
  ('robe', $d$A plain long robe of undyed wool, belted at the waist. Monastic, scholarly or simply cheap, depending on who is wearing it.$d$),
  ('the_ember', $d$A warm shard of red stone that never cools, small enough to close a fist around. It answers a spell the way a dry wick answers a flame.$d$),
  ('tinderbox', $d$A small tin of flint, steel and char cloth. Enough to light a torch quickly and anything else with patience.$d$),

  /* ----------------------------- loot ------------------------------ */
  ('arrow', $d$A shaft of straight-grained wood, fletched with three feathers and tipped with a narrow bodkin point.$d$),
  ('blanket', $d$Coarse heavy wool. Worth more on a cold night than most things costing ten times as much.$d$),
  ('coin_cp', $d$A copper piece, soft-edged and worn smooth by handling. Ten make a silver.$d$),
  ('coin_sp', $d$A silver piece, the ordinary coin of daily trade. Ten make a gold.$d$),
  ('coin_gp', $d$A gold piece, the coin of wages and contracts. Most common folk see few of them in a year.$d$),
  ('coin_pp', $d$A platinum piece, pale and heavy. Worth ten gold, and spent so rarely that producing one invites questions.$d$),

  /* ---------------------------- weapons ---------------------------- */
  ('bardiche', $d$A long cleaving blade bolted to a tall haft at two points, so the whole edge swings as one piece. Made where steel was dear and timber was not.$d$),
  ('battleaxe', $d$A single broad crescent on a straight haft, balanced for one hand or two. The working weapon of a soldier who could not afford a sword.$d$),
  ('bearded_axe', $d$The lower edge drawn down into a hook - the beard - which catches a shield rim and drags it aside. A carpenter''s tool and a raider''s weapon, often the same axe.$d$),
  ('bec_de_corbin', $d$The crow''s beak: a heavy spike backed by a hammer face on a long shaft, built to crush plate rather than cut it. Where armour got better, this was the answer.$d$),
  ('boar_spear', $d$A broad leaf-shaped head with a crossbar below it, so that whatever is impaled cannot run up the shaft. It works on things other than boar.$d$),
  ('bolas', $d$Three weighted cords joined at one end, thrown to wrap and tangle the legs. Hunters'' work, and cheap.$d$),
  ('club', $d$A length of hard wood, thicker at one end. The oldest weapon there is and still a persuasive one.$d$),
  ('crossbow_hand', $d$A small steel-prodded crossbow drawn and loosed in one hand, concealable under a cloak. Illegal in most civilised cities, which has never slowed its sale.$d$),
  ('crossbow_light', $d$A wooden stock and steel prod with a stirrup for the foot. Anyone can be taught to use one in an afternoon, which is why lords dislike them.$d$),
  ('dagger', $d$A short double-edged blade for close work, worn at the belt or in a boot. Every traveller carries one; not all of them for eating.$d$),
  ('dart', $d$A weighted throwing spike with a short flight, thrown underhand in a flurry.$d$),
  ('estoc', $d$A long sword with no cutting edge at all - a stiff four-sided bar tapering to a point, made to find the gaps in a harness. Gripped by the blade when it comes to that.$d$),
  ('falchion', $d$A heavy single-edged sword that broadens toward the tip, cutting like a cleaver and costing far less to make than a longsword.$d$),
  ('flail', $d$A striking head on a short chain. It goes around a raised shield instead of into it, and it is nearly as dangerous to its wielder.$d$),
  ('flanged_mace', $d$A mace whose head bears raised vanes, driving the whole blow into a narrow line. It splits mail where a smooth head would only bruise.$d$),
  ('francisca', $d$A short throwing axe with a deeply curved head, loosed in a volley just before the lines meet. It tumbles in flight and wrecks a shield even when it does not bite.$d$),
  ('gladius', $d$A short broad stabbing sword for fighting shoulder to shoulder behind a shield. Unremarkable alone and devastating in a line.$d$),
  ('glaive', $d$A single-edged blade the length of a forearm on a tall pole. It keeps the fight at the far end of the shaft, where it belongs.$d$),
  ('greataxe', $d$A great crescent head on a long haft, swung with the whole body. Nothing about it is subtle and nothing needs to be.$d$),
  ('greatclub', $d$A two-handed length of knotted hardwood - sometimes a branch barely worked at all.$d$),
  ('greatsword', $d$A long double-edged blade with a two-hand grip and a heavy pommel to balance it. A weapon that needs room, training and a reason.$d$),
  ('guisarme', $d$A curved hooked blade on a long shaft, grown out of a pruning hook. It was made for pulling riders off horses and still is.$d$),
  ('halberd', $d$An axe blade, a top spike and a rear hook on one shaft - it cuts, thrusts and pulls. The most useful thing a footman ever carried.$d$),
  ('handaxe', $d$A one-handed axe light enough to throw, and good for firewood when nobody is trying to kill you.$d$),
  ('heavy_crossbow', $d$A thick steel prod drawn with a windlass, throwing a bolt hard enough to punch plate. Slow to load - it only has to be right once.$d$),
  ('horsemans_pick', $d$A short-hafted spike for use from the saddle, driven downward through helm and mail.$d$),
  ('javelin', $d$A light throwing spear with a slim iron head, carried in bundles and thrown before the charge.$d$),
  ('khopesh', $d$A sickle-sword out of an older age, the edge on the outside of the curve and a hook inside it that strips a shield from a hand.$d$),
  ('lance', $d$A long couched spear with a hand guard, made to be used at a gallop and awkward anywhere else.$d$),
  ('light_hammer', $d$A one-handed smith''s hammer, equally at home on an anvil or a skull.$d$),
  ('longbow', $d$A tall stave of yew drawn to the ear. Years of practice to use well, and nothing on a field reaches further for less.$d$),
  ('longsword', $d$A straight double-edged blade with a cross guard and a grip long enough for a second hand. The knight''s weapon, and the sign of one.$d$),
  ('mace', $d$A weighted metal head on a short haft, built to break what is under the armour without having to cut through it.$d$),
  ('mace_of_the_deep_song', $d$An old mace of dark grey stone, far heavier than its size explains. Struck, it hums for a long time afterwards.$d$),
  ('maul', $d$A two-handed sledge with a steel head. It breaks bones through plate and does not much care how good the plate is.$d$),
  ('military_fork', $d$Two long tines on a pole - out of a hayfork and into a war. It turns a polearm aside and pins a man as readily as a sheaf.$d$),
  ('morningstar', $d$A spiked head fixed solid to the shaft, with no chain. The spikes make it a piercing weapon and the rigid shaft makes it one you can aim.$d$),
  ('net', $d$A weighted mesh thrown to entangle. A gladiator''s tool, and a trapper''s.$d$),
  ('pike', $d$A very long ash shaft tipped with a narrow steel head. One is a nuisance; a hundred held level is a wall no horse will charge.$d$),
  ('quarterstaff', $d$A plain pole of hardwood, often iron-shod at both ends. The commonest weapon in the world, because it is also just a walking stick.$d$),
  ('ranseur', $d$A spear head with two curved side lugs that catch another weapon and twist it aside. Made for breaking a pike line from the flank.$d$),
  ('rapier', $d$A long slender thrusting sword with a swept guard. Quick, precise, and a civilian''s weapon right up until somebody is killed with it.$d$),
  ('scimitar', $d$A curved single-edged sword that cuts on the draw. Light enough to carry all day, which is why cavalry favour it.$d$),
  ('seax', $d$A heavy single-edged knife the length of a forearm, worn flat across the belt. A tool first, and a weapon when it has to be.$d$),
  ('shortbow', $d$A short bow drawn to the chest. Usable from horseback and in a wood, where a longbow is no use at all.$d$),
  ('shortsword', $d$A broad leaf-bladed sword for thrusting at close quarters. The weapon that goes with a shield - or with a second shortsword.$d$),
  ('sickle', $d$A curved harvesting blade on a short handle. It is a farm tool until the harvest is over.$d$),
  ('sling', $d$A leather cradle on two cords, whirled and released. Almost weightless, free to carry, and its ammunition is lying on the ground.$d$),
  ('spear', $d$An ash shaft with a leaf-shaped head, used one-handed behind a shield or in both hands for reach. More people have been killed with this than with anything else.$d$),
  ('staff_sling', $d$A sling mounted on a short staff, trading accuracy for a long arcing throw. It lobs over a shield wall rather than through it.$d$),
  ('trident', $d$A three-tined fishing spear taken up as a weapon, as much at home in the water as out of it.$d$),
  ('voulge', $d$A heavy cleaving blade lashed flat against a long shaft, chopping rather than cutting. Crude to make and difficult to stand in front of.$d$),
  ('war_flail', $d$A two-handed haft with a long chained head. Enormous reach around a shield, and almost no control once it is moving.$d$),
  ('war_pick', $d$A narrow back-curved spike on a short haft, meant to punch through armour at one small point.$d$),
  ('war_scythe', $d$A harvesting blade reforged to stand straight on its shaft instead of across it. The weapon of a peasant rising, and better than it has any right to be.$d$),
  ('warhammer', $d$A compact steel hammer with a spike opposite the face, used in one hand or two. It is made for armour and for very little else.$d$),
  ('whip', $d$A long braided leather lash. It reaches further than a sword and hurts a great deal, and it will not get through so much as a padded coat.$d$)
) as v(k, d)
where i.key = v.k
  and i.game_id is null
  and i.description is null;
