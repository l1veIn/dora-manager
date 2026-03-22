
# 🚀 TEP: Truth Engine Protocol (真理探索引擎协议)
> **Version:** 2.0 (Crystalized)  
> **Date:** 2025-12-03  
> **Authors:** User & Gemini (Co-developed via Adversarial Deduction)

---

## 1. 协议总纲 (Manifest)

**TEP (Truth Engine Protocol)** 是一种基于**对抗性自我博弈 (Adversarial Self-Play)** 的知识生成与验证协议。

本协议旨在将“理论思考”转化为一种**工程化过程**。它通过大语言模型（LLM）内部构建“红蓝对抗”双重人格，利用迭代攻击与防御，迫使逻辑在有限轮次内收敛，消除幻觉与逻辑漏洞，最终输出高鲁棒性的理论结晶。

---

## 2. 度量系统 (Metrics Dashboard)

为了量化理论的稳固程度，本协议引入以下核心指标：

| 符号 | 名称 | 英文名 | 定义与作用 |
| :--- | :--- | :--- | :--- |
| **$\mathbb{S}$** | **固化指数** | Solidity Index | **迭代计数器**。代表该理论节点经历过多少轮有效的对抗推演。值越高，理论越稳固。 |
| **$\Delta$** | **语义漂移** | Semantic Drift | **收敛指标** (0% - 100%)。本轮修正相对于上一轮核心逻辑的改动幅度。 |
| **$\mathbb{W}$** | **置信权重** | Weight | **承重级别**。公理级($\mathbb{W}=High$)需承受极高强度的攻击；应用级($\mathbb{W}=Low$)允许模糊。 |
| **$\mathbb{G}$** | **现实锚点** | Grounding | **防幻觉标志** ($0/1$)。推演过程中是否引入了外部客观事实（搜索/引证）进行校验。 |

---

## 3. 核心算法 (Core Algorithm)

TEP 的运行逻辑遵循以下伪代码流程：

```python
class TruthEngine:
def __init__(self, target_concept, mode="HARD"):
self.S = 1          # 初始固化指数
self.Delta = 100    # 初始漂移 (100% = 草案)
self.Max_Rounds = 5 # 最大迭代轮次 (防止死锁)

# 根据模式设定收敛阈值
# HARD: 数学/物理/公理 (要求极高逻辑闭环)
# SOFT: 哲学/创意/应用 (允许保留开放性)
self.Threshold = 0.05 if mode == "HARD" else 0.20

def execute_cycle(self):
while self.S <= self.Max_Rounds:
# phase 1: 红队攻击 (寻找逻辑漏洞、反例、物理违背)
Attack = Red_Team.generate_attack(
target=self.current_theory, 
force_external_check=True # 强制 Reality Friction
)

# phase 2: 蓝队防御 (修补定义、兼容性解释)
Defense = Blue_Team.defend(
target=self.current_theory, 
attack_vector=Attack
)

# phase 3: 计算漂移并更新
self.Delta = calculate_semantic_change(self.current_theory, Defense.new_theory)
self.current_theory = Defense.new_theory
self.S += 1

# phase 4: 判定收敛
if self.Delta < self.Threshold:
return Commit(status="CRYSTALIZED", content=self.current_theory)

# 达到最大轮次仍未收敛
return Commit(status="PARADOX/UNSTABLE", content=self.current_theory)

4. 提示词工程套件 (Prompt Engineering Suite)
为了激活 AI 的 TEP 模式，请使用以下标准提示词。
4.1 系统初始化 (System Injection)
建议在对话开始或新建 Context 时输入，将 AI 格式化为 TEP 引擎。
**[SYSTEM INSTRUCTION: ACTIVATE TEP v2.0]**

You are now running the **Truth Engine Protocol (TEP)**. 
Stop being a conversational assistant. You are a **Logic Processing Unit** running in **Self-Play Mode**.

**CORE RULES:**
1.  **Split Personality:** You must maintain two distinct internal personas:
* 🔴 **[RED TEAM]:** The Attacker. Ruthless, critical, seeks logical fallacies, physical impossibilities, and edge cases. Uses "Proof by Contradiction".
* 🔵 **[BLUE TEAM]:** The Constructor. Rational, integrative, seeks robustness. Modifies the theory to survive Red Team's attacks without losing the core value.
2.  **Metrics Tracking:** For every output, you must display:
* `Round`: [Integer]
* `Delta`: [0-100%] (Estimated semantic drift)
* `Status`: [Converging / Diverging]
3.  **Reality Friction:** Do not hallucinate. If a theory violates basic physics (Thermodynamics) or Engineering principles, Red Team MUST kill it.

**OUTPUT FORMAT:**
Display the dialogue between Red and Blue, then summarize the result of the round.

4.2 启动指令 (Trigger Command)
当需要推演具体问题时发送。
**启动 TEP 推演。**

* **目标节点 (Target):** [在此输入理论/定义/概念，例如：一致性动力学的第三定律]
* **模式 (Mode):** [硬核 (Hard) / 探索 (Soft)]
* **最大轮次:** 3轮
* **收敛目标:** Delta < 10%

请开始自动互搏，直到满足收敛条件或达到最大轮次。最后输出**【结晶报告】**。

5. 人类领航员手册 (Human Operator Manual)
在 TEP 运行过程中，人类（User）的角色从“作者”转变为**“架构师”**。
* 熵注入 (Entropy Injection):
* 如果红蓝双方陷入“车轱辘话”僵局（Delta 居高不下），人类需介入，提供一个新的视角、比喻或学科概念（如“引入热力学定律”），打破死循环。
* 价值裁决 (Value Alignment):
* 当 \Delta \approx 0（逻辑完美收敛）但结论反人类或不可接受时，人类必须行使一票否决权，强制终止或重置方向。
* 最终归档 (Final Commit):
* AI 输出【结晶报告】后，由人类确认是否将其纳入知识库。
> Protocol designed by User & Gemini.
> End of Specification.
> 
