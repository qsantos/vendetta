#include "universe.h"

#include "../util.h"

universe_t* universe_init(graphics_t* g)
{
	universe_t* u = CALLOC(universe_t, 1);

	u->n_materials = 26;
	u->materials[ 0].name = "Herbs";
	u->materials[ 1].name = "Apples";
	u->materials[ 2].name = "Wood";
	u->materials[ 3].name = "Planks";
	u->materials[ 4].name = "Wool";
	u->materials[ 5].name = "Cattle";
	u->materials[ 6].name = "Meat";
	u->materials[ 7].name = "Wheat";
	u->materials[ 8].name = "Bread";
	u->materials[ 9].name = "Barley";
	u->materials[10].name = "Beer";
	u->materials[11].name = "Leather";
	u->materials[12].name = "Stone";
	u->materials[13].name = "Bricks";
	u->materials[14].name = "Iron";
	u->materials[15].name = "Pig iron";
	u->materials[16].name = "Steel";
	u->materials[17].name = "Crystals";
	u->materials[18].name = "Health potions";
	u->materials[19].name = "Ether";
	u->materials[20].name = "Parchment";
	u->materials[21].name = "Paper";
	u->materials[22].name = "Cider";
	u->materials[23].name = "Gold";
	u->materials[24].name = "Ingot";
	u->materials[25].name = "Sulphur";

	u->n_mines = 8;
	u->mines[0] = (kindOf_mine_t){0, "Herbs",            0, &u->materials[ 0]};
	u->mines[1] = (kindOf_mine_t){1, "Apple tree",       1, &u->materials[ 1]};
	u->mines[2] = (kindOf_mine_t){2, "Tree",             2, &u->materials[ 2]};
	u->mines[3] = (kindOf_mine_t){3, "Rocks",           12 ,&u->materials[12]};
	u->mines[4] = (kindOf_mine_t){4, "Iron vein",       14, &u->materials[14]};
	u->mines[5] = (kindOf_mine_t){5, "Crystals",        17, &u->materials[17]};
	u->mines[6] = (kindOf_mine_t){6, "Gold ore",        23, &u->materials[23]};
	u->mines[7] = (kindOf_mine_t){7, "Sulphur deposit", 25, &u->materials[25]};

	u->n_buildings = 49;
	kindOf_building_init(&u->buildings[ 0], g, "Wheat farm");
	kindOf_building_init(&u->buildings[ 1], g, "Barley farm");
	kindOf_building_init(&u->buildings[ 2], g, "Sawmill");
	kindOf_building_init(&u->buildings[ 3], g, "Sheepfold");
	kindOf_building_init(&u->buildings[ 4], g, "Cattle farm");
	kindOf_building_init(&u->buildings[ 5], g, "Butchery");
	kindOf_building_init(&u->buildings[ 6], g, "Bakery");
	kindOf_building_init(&u->buildings[ 7], g, "Tavern");
	kindOf_building_init(&u->buildings[ 8], g, "Clothing shop");
	kindOf_building_init(&u->buildings[ 9], g, "Workshop");
	kindOf_building_init(&u->buildings[10], g, "Store");
	kindOf_building_init(&u->buildings[11], g, "Tannery");
	kindOf_building_init(&u->buildings[12], g, "Shoe maker");
	kindOf_building_init(&u->buildings[13], g, "Glove maker");
	kindOf_building_init(&u->buildings[14], g, "Wheat barn");
	kindOf_building_init(&u->buildings[15], g, "Barley barn");
	kindOf_building_init(&u->buildings[16], g, "Stable");
	kindOf_building_init(&u->buildings[17], g, "Quarry");
	kindOf_building_init(&u->buildings[18], g, "Barracks");
	kindOf_building_init(&u->buildings[19], g, "Foundry");
	kindOf_building_init(&u->buildings[20], g, "Smithy");
	kindOf_building_init(&u->buildings[21], g, "Archery");
	kindOf_building_init(&u->buildings[22], g, "Armory");
	kindOf_building_init(&u->buildings[23], g, "Steelworks");
	kindOf_building_init(&u->buildings[24], g, "Parchmentery");
	kindOf_building_init(&u->buildings[25], g, "Church");
	kindOf_building_init(&u->buildings[26], g, "Laboratory");
	kindOf_building_init(&u->buildings[27], g, "Ether lab");
	kindOf_building_init(&u->buildings[28], g, "Magic guild");
	kindOf_building_init(&u->buildings[29], g, "Pyromagic school");
	kindOf_building_init(&u->buildings[30], g, "Necropolis");
	kindOf_building_init(&u->buildings[31], g, "Aquavitae school");
	kindOf_building_init(&u->buildings[32], g, "Stationer");
	kindOf_building_init(&u->buildings[33], g, "Goldsmith");
	kindOf_building_init(&u->buildings[34], g, "Cider-house");
	kindOf_building_init(&u->buildings[35], g, "Jeweller");
	kindOf_building_init(&u->buildings[36], g, "Arsenal");
	kindOf_building_init(&u->buildings[37], g, "Tectonic workshop");
	kindOf_building_init(&u->buildings[38], g, "Chaos guild");
	kindOf_building_init(&u->buildings[39], g, "Order guild");
	kindOf_building_init(&u->buildings[40], g, "Elixir lab");
	kindOf_building_init(&u->buildings[41], g, "Imperial smith");
	kindOf_building_init(&u->buildings[42], g, "Imperial armory");
	kindOf_building_init(&u->buildings[43], g, "Headquarters");
	kindOf_building_init(&u->buildings[44], g, "Dragon nest");
	kindOf_building_init(&u->buildings[45], g, "Guard tower");
	kindOf_building_init(&u->buildings[46], g, "Market");
	kindOf_building_init(&u->buildings[47], g, "Palace");
	kindOf_building_init(&u->buildings[48], g, "Cantonment");

	return u;
}

void universe_exit(universe_t* u)
{
	for (int i = 0; i < u->n_buildings; i++)
		kindOf_building_exit(&u->buildings[i]);

	free(u);
}
