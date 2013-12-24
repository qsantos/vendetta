#include "mine.h"

#include <stdlib.h>

void kindOf_mine_init(kindOf_mine_t* m, wchar_t* name)
{
	m->name = name;
	components_init(&m->harvest);
}

void kindOf_mine_exit(kindOf_mine_t* m)
{
	free(m->name);
}
