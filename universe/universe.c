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
	u->buildings[ 0].name = "Wheat farm";
	u->buildings[ 1].name = "Barley farm";
	u->buildings[ 2].name = "Sawmill";
	u->buildings[ 3].name = "Sheepfold";
	u->buildings[ 4].name = "Cattle farm";
	u->buildings[ 5].name = "Butchery";
	u->buildings[ 6].name = "Bakery";
	u->buildings[ 7].name = "Tavern";
	u->buildings[ 8].name = "Clothing shop";
	u->buildings[ 9].name = "Workshop";
	u->buildings[10].name = "Store";
	u->buildings[11].name = "Tannery";
	u->buildings[12].name = "Shoe maker";
	u->buildings[13].name = "Glove maker";
	u->buildings[14].name = "Wheat barn";
	u->buildings[15].name = "Barley barn";
	u->buildings[16].name = "Stable";
	u->buildings[17].name = "Quarry";
	u->buildings[18].name = "Barracks";
	u->buildings[19].name = "Foundry";
	u->buildings[20].name = "Smithy";
	u->buildings[21].name = "Archery";
	u->buildings[22].name = "Armory";
	u->buildings[23].name = "Steelworks";
	u->buildings[24].name = "Parchmentery";
	u->buildings[25].name = "Church";
	u->buildings[26].name = "Laboratory";
	u->buildings[27].name = "Ether lab";
	u->buildings[28].name = "Magic guild";
	u->buildings[29].name = "Pyromagic school";
	u->buildings[30].name = "Necropolis";
	u->buildings[31].name = "Aquavitae school";
	u->buildings[32].name = "Stationer";
	u->buildings[33].name = "Goldsmith";
	u->buildings[34].name = "Cider-house";
	u->buildings[35].name = "Jeweller";
	u->buildings[36].name = "Arsenal";
	u->buildings[37].name = "Tectonic workshop";
	u->buildings[38].name = "Chaos guild";
	u->buildings[39].name = "Order guild";
	u->buildings[40].name = "Elixir lab";
	u->buildings[41].name = "Imperial smith";
	u->buildings[42].name = "Imperial armory";
	u->buildings[43].name = "Headquarters";
	u->buildings[44].name = "Dragon nest";
	u->buildings[45].name = "Guard tower";
	u->buildings[46].name = "Market";
	u->buildings[47].name = "Palace";
	u->buildings[48].name = "Cantonment";

	for (int i = 0; i < u->n_buildings; i++)
	{
		char s[1024];
		snprintf(s, 1024, "buildings/%s.png", u->buildings[i].name);
		u->buildings[i].sprite = graphics_spriteForImg(g, s);
	}

	return u;
}

void universe_exit(universe_t* u)
{
	free(u);
}
