#ifndef U_CHARACTER_H
#define U_CHARACTER_H

typedef struct kindOf_character kindOf_character_t;

struct kindOf_character
{
	int sprite;
};

void kindOf_character_init(kindOf_character_t* c);
void kindOf_character_exit(kindOf_character_t* c);

#endif
