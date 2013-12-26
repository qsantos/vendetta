#include "item.h"

#include <stdlib.h>

void kindOf_item_init(kindOf_item_t* i)
{
	i->name = NULL;
	i->skill = 0;
	i->category = 0;
	for (int k = 0; k < N_SPECIAL_SKILLS; k++)
		i->bonus_special[k] = 0;
}

void kindOf_item_exit(kindOf_item_t* i)
{
	free(i->name);
}
