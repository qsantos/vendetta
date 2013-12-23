#ifndef U_MINE_H
#define U_MINE_H

typedef struct kindOf_mine kindOf_mine_t;

#include <wchar.h>

struct kindOf_mine
{
	int id;
	wchar_t* name;

	int material_id;
};

void kindOf_mine_exit(kindOf_mine_t* m);

#endif
