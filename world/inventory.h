#ifndef W_INVENTORY_H
#define W_INVENTORY_H

typedef struct inventory inventory_t;

#include "../universe/universe.h"

struct inventory
{
	float* materials;
	float* items;
};

void inventory_init(inventory_t* i, universe_t* u);
void inventory_exit(inventory_t* i);

#endif
