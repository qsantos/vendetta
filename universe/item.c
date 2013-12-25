#include "item.h"

#include <stdlib.h>

void kindOf_item_init(kindOf_item_t* i)
{
	i->name = NULL;
}

void kindOf_item_exit(kindOf_item_t* i)
{
	free(i->name);
}
