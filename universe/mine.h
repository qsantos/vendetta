#ifndef U_MINE_H
#define U_MINE_H

typedef struct kindOf_mine kindOf_mine_t;

#include <wchar.h>

#include "component.h"

struct kindOf_mine
{
	int id;
	wchar_t* name;

	components_t harvest;
};

void kindOf_mine_init(kindOf_mine_t* m, wchar_t* name);
void kindOf_mine_exit(kindOf_mine_t* m);

#endif
