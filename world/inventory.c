#include "inventory.h"

#include <string.h>

#include "../util.h"

void inventory_init(inventory_t* i, universe_t* u)
{
	i->materials = CALLOC(float, u->n_materials);
	i->items     = CALLOC(float, u->n_items);

	memset(i->materials, 0, sizeof(float)*u->n_materials);
	memset(i->items,     0, sizeof(float)*u->n_items);
}

void inventory_exit(inventory_t* i)
{
	free(i->items);
	free(i->materials);
}
