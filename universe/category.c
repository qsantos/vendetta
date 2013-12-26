#include "category.h"

#include <stdlib.h>

void kindOf_category_init(kindOf_category_t* c)
{
	c->name = NULL;
}

void kindOf_category_exit(kindOf_category_t* c)
{
	free(c->name);
}
