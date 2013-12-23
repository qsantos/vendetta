#include "universe.h"

#include "../util.h"

universe_t* universe_init(graphics_t* g)
{
	universe_t* u = CALLOC(universe_t, 1);

	u->n_materials = 26;
	u->materials[ 0].name = L"Herbs";
	u->materials[ 1].name = L"Apples";
	u->materials[ 2].name = L"Wood";
	u->materials[ 3].name = L"Planks";
	u->materials[ 4].name = L"Wool";
	u->materials[ 5].name = L"Cattle";
	u->materials[ 6].name = L"Meat";
	u->materials[ 7].name = L"Wheat";
	u->materials[ 8].name = L"Bread";
	u->materials[ 9].name = L"Barley";
	u->materials[10].name = L"Beer";
	u->materials[11].name = L"Leather";
	u->materials[12].name = L"Stone";
	u->materials[13].name = L"Bricks";
	u->materials[14].name = L"Iron";
	u->materials[15].name = L"Pig iron";
	u->materials[16].name = L"Steel";
	u->materials[17].name = L"Crystals";
	u->materials[18].name = L"Health potions";
	u->materials[19].name = L"Ether";
	u->materials[20].name = L"Parchment";
	u->materials[21].name = L"Paper";
	u->materials[22].name = L"Cider";
	u->materials[23].name = L"Gold";
	u->materials[24].name = L"Ingot";
	u->materials[25].name = L"Sulphur";

	u->n_items = 5;
	u->items[0].name = L"Working clothes";
	u->items[1].name = L"Luxury clothing";
	u->items[2].name = L"Hat";
	u->items[3].name = L"Axe";
	u->items[4].name = L"Hammer";

	u->n_mines = 8;
	u->mines[0] = (kindOf_mine_t){0, L"Herbs",            0, &u->materials[ 0]};
	u->mines[1] = (kindOf_mine_t){1, L"Apple tree",       1, &u->materials[ 1]};
	u->mines[2] = (kindOf_mine_t){2, L"Tree",             2, &u->materials[ 2]};
	u->mines[3] = (kindOf_mine_t){3, L"Rocks",           12 ,&u->materials[12]};
	u->mines[4] = (kindOf_mine_t){4, L"Iron vein",       14, &u->materials[14]};
	u->mines[5] = (kindOf_mine_t){5, L"Crystals",        17, &u->materials[17]};
	u->mines[6] = (kindOf_mine_t){6, L"Gold ore",        23, &u->materials[23]};
	u->mines[7] = (kindOf_mine_t){7, L"Sulphur deposit", 25, &u->materials[25]};

	u->n_buildings = 49;
	kindOf_building_init(&u->buildings[ 0], g, L"Wheat farm");
	kindOf_building_init(&u->buildings[ 1], g, L"Barley farm");
	kindOf_building_init(&u->buildings[ 2], g, L"Sawmill");
	kindOf_building_init(&u->buildings[ 3], g, L"Sheepfold");
	kindOf_building_init(&u->buildings[ 4], g, L"Cattle farm");
	kindOf_building_init(&u->buildings[ 5], g, L"Butchery");
	kindOf_building_init(&u->buildings[ 6], g, L"Bakery");
	kindOf_building_init(&u->buildings[ 7], g, L"Tavern");
	kindOf_building_init(&u->buildings[ 8], g, L"Clothing shop");
	kindOf_building_init(&u->buildings[ 9], g, L"Workshop");
	kindOf_building_init(&u->buildings[10], g, L"Store");
	kindOf_building_init(&u->buildings[11], g, L"Tannery");
	kindOf_building_init(&u->buildings[12], g, L"Shoe maker");
	kindOf_building_init(&u->buildings[13], g, L"Glove maker");
	kindOf_building_init(&u->buildings[14], g, L"Wheat barn");
	kindOf_building_init(&u->buildings[15], g, L"Barley barn");
	kindOf_building_init(&u->buildings[16], g, L"Stable");
	kindOf_building_init(&u->buildings[17], g, L"Quarry");
	kindOf_building_init(&u->buildings[18], g, L"Barracks");
	kindOf_building_init(&u->buildings[19], g, L"Foundry");
	kindOf_building_init(&u->buildings[20], g, L"Smithy");
	kindOf_building_init(&u->buildings[21], g, L"Archery");
	kindOf_building_init(&u->buildings[22], g, L"Armory");
	kindOf_building_init(&u->buildings[23], g, L"Steelworks");
	kindOf_building_init(&u->buildings[24], g, L"Parchmentery");
	kindOf_building_init(&u->buildings[25], g, L"Church");
	kindOf_building_init(&u->buildings[26], g, L"Laboratory");
	kindOf_building_init(&u->buildings[27], g, L"Ether lab");
	kindOf_building_init(&u->buildings[28], g, L"Magic guild");
	kindOf_building_init(&u->buildings[29], g, L"Pyromagic school");
	kindOf_building_init(&u->buildings[30], g, L"Necropolis");
	kindOf_building_init(&u->buildings[31], g, L"Aquavitae school");
	kindOf_building_init(&u->buildings[32], g, L"Stationer");
	kindOf_building_init(&u->buildings[33], g, L"Goldsmith");
	kindOf_building_init(&u->buildings[34], g, L"Cider-house");
	kindOf_building_init(&u->buildings[35], g, L"Jeweller");
	kindOf_building_init(&u->buildings[36], g, L"Arsenal");
	kindOf_building_init(&u->buildings[37], g, L"Tectonic workshop");
	kindOf_building_init(&u->buildings[38], g, L"Chaos guild");
	kindOf_building_init(&u->buildings[39], g, L"Order guild");
	kindOf_building_init(&u->buildings[40], g, L"Elixir lab");
	kindOf_building_init(&u->buildings[41], g, L"Imperial smith");
	kindOf_building_init(&u->buildings[42], g, L"Imperial armory");
	kindOf_building_init(&u->buildings[43], g, L"Headquarters");
	kindOf_building_init(&u->buildings[44], g, L"Dragon nest");
	kindOf_building_init(&u->buildings[45], g, L"Guard tower");
	kindOf_building_init(&u->buildings[46], g, L"Market");
	kindOf_building_init(&u->buildings[47], g, L"Palace");
	kindOf_building_init(&u->buildings[48], g, L"Cantonment");

	return u;
}

void universe_exit(universe_t* u)
{
	for (int i = 0; i < u->n_buildings; i++)
		kindOf_building_exit(&u->buildings[i]);

	free(u);
}
