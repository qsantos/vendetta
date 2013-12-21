#include "universe.h"

void universe_init(universe_t* u)
{
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
	u->mines[0] = (kindOf_mine_t){"Herbs",            0, &u->materials[ 0]};
	u->mines[1] = (kindOf_mine_t){"Apple tree",       1, &u->materials[ 1]};
	u->mines[2] = (kindOf_mine_t){"Tree",             2, &u->materials[ 2]};
	u->mines[3] = (kindOf_mine_t){"Quarry",          12 ,&u->materials[12]};
	u->mines[4] = (kindOf_mine_t){"Iron ore",        14, &u->materials[14]};
	u->mines[5] = (kindOf_mine_t){"Crystals",        17, &u->materials[17]};
	u->mines[6] = (kindOf_mine_t){"Gold ore",        23, &u->materials[23]};
	u->mines[7] = (kindOf_mine_t){"Sulphur deposit", 25, &u->materials[25]};
}
