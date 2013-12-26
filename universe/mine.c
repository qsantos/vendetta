#include "mine.h"

#include <stdlib.h>

void kindOf_mine_init(kindOf_mine_t* m)
{
	m->name = NULL;
	transform_init(&m->harvest);
}

void kindOf_mine_exit(kindOf_mine_t* m)
{
	free(m->name);
}
