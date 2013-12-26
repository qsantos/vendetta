#ifndef U_CATEGORY_H
#define U_CATEGORY_H

typedef struct kindOf_category kindOf_category_t;

#include <wchar.h>

struct kindOf_category
{
	wchar_t* name;
};

void kindOf_category_init(kindOf_category_t* c);
void kindOf_category_exit(kindOf_category_t* c);

#endif
