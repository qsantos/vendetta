#include "item.h"

#include "../util.h"

void kindOf_item_init(kindOf_item_t* i, universe_t* u)
{
	i->name = NULL;
	i->skill = 0;
	i->category = 0;

	for (int k = 0; k < N_SPECIAL_SKILLS; k++)
		i->bonus_special[k] = 0;

	i->bonus_material = CALLOC(float, u->n_materials);
	for (int k = 0; k < u->n_materials; k++)
		i->bonus_material[k] = 0;

	i->bonus_item = CALLOC(float, u->n_items);
	for (int k = 0; k < u->n_items; k++)
		i->bonus_item[k] = 0;
}

void kindOf_item_exit(kindOf_item_t* i)
{
	free(i->bonus_material);
	free(i->bonus_item);
	free(i->name);
}
